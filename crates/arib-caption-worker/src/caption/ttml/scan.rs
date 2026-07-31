use crate::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TsFraming {
    pub(crate) packet_size: usize,
    pub(crate) ts_offset: usize,
}

pub(crate) fn scan_ts_ttml_impl<F, P, C, R>(
    path: &Path,
    tracks: &DataTracks,
    framing: TsFraming,
    mut on_caption: F,
    mut on_progress: P,
    mut cancelled: C,
    mut on_pes: R,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut packet = vec![0u8; framing.packet_size];
    let mut pes: HashMap<u16, Vec<u8>> = tracks.pids.iter().map(|pid| (*pid, Vec::new())).collect();
    let mut timeline_origin_ms = None;
    let mut pes_offsets: HashMap<u16, u64> = HashMap::new();
    let mut summary = B24DecodeSummary::default();
    let mut next_progress = PROGRESS_INTERVAL;
    loop {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "conversion cancelled",
            ));
        }
        match reader.read_exact(&mut packet) {
            Ok(()) => summary.bytes_read += framing.packet_size as u64,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        }
        if summary.bytes_read >= next_progress {
            on_progress(&summary);
            next_progress += PROGRESS_INTERVAL;
        }
        let Some((pid, payload_start, payload)) = ts_payload(&packet[framing.ts_offset..]) else {
            continue;
        };
        if timeline_origin_ms.is_none() && payload_start {
            timeline_origin_ms = pes_pts_from_header(payload);
        }
        let Some(buffer) = pes.get_mut(&pid) else {
            continue;
        };
        if payload_start && !buffer.is_empty() {
            on_pes(
                pid,
                *pes_offsets.get(&pid).unwrap_or(&summary.bytes_read),
                buffer,
            )?;
            let pts = pes_pts_from_header(buffer)
                .map(|value| normalise_pts(value, timeline_origin_ms.unwrap_or(value)))
                .unwrap_or(0);
            for document in ttml_documents(buffer) {
                for caption in parse_ttml_captions(&document.xml, pts) {
                    summary.captions += 1;
                    summary.characters += caption.text.chars().count() as u64;
                    on_caption(caption)?;
                }
            }
            summary.pes_packets += 1;
            buffer.clear();
        }
        if buffer.len() + payload.len() > PES_BUFFER_LIMIT {
            buffer.clear();
            summary.decoder_errors += 1;
            continue;
        }
        if buffer.is_empty() {
            pes_offsets.insert(
                pid,
                summary
                    .bytes_read
                    .saturating_sub(framing.packet_size as u64),
            );
        }
        buffer.extend_from_slice(payload);
    }
    for (pid, buffer) in pes {
        if buffer.is_empty() {
            continue;
        }
        on_pes(
            pid,
            *pes_offsets.get(&pid).unwrap_or(&summary.bytes_read),
            &buffer,
        )?;
        let pts = pes_pts_from_header(&buffer)
            .map(|value| normalise_pts(value, timeline_origin_ms.unwrap_or(value)))
            .unwrap_or(0);
        for document in ttml_documents(&buffer) {
            for caption in parse_ttml_captions(&document.xml, pts) {
                summary.captions += 1;
                summary.characters += caption.text.chars().count() as u64;
                on_caption(caption)?;
            }
        }
        summary.pes_packets += 1;
    }
    Ok(summary)
}
