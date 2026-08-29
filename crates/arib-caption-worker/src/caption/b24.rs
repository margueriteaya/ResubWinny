use crate::*;

pub(crate) fn b24_payload_from_pes(pes: &[u8]) -> Option<(&[u8], Option<Pts90k>)> {
    if pes.len() < 9 || pes[..4] != [0, 0, 1, 0xbd] {
        return None;
    }
    let payload_start = 9 + usize::from(pes[8]);
    let payload = pes.get(payload_start..)?;
    if payload.is_empty() {
        return None;
    }
    Some((payload, pes_pts_from_header(pes)))
}

pub(crate) fn pes_pts_from_header(pes: &[u8]) -> Option<Pts90k> {
    if pes.len() < 14 || pes[..3] != [0, 0, 1] || pes[7] & 0x80 == 0 {
        return None;
    }
    // ISO/IEC 13818-1 requires the PTS prefix and all three marker bits.
    // Some recorder-private subtitle PES packets set PTS_DTS_flags while
    // storing five zero bytes; accepting those bytes collapses every caption
    // onto t=0 and hides the transport timestamp that can actually time it.
    if !matches!(pes[9] & 0xf0, 0x20 | 0x30)
        || pes[9] & 1 == 0
        || pes[11] & 1 == 0
        || pes[13] & 1 == 0
    {
        return None;
    }
    let value = (u64::from(pes[9] & 0x0e) << 29)
        | (u64::from(pes[10]) << 22)
        | (u64::from(pes[11] & 0xfe) << 14)
        | (u64::from(pes[12]) << 7)
        | u64::from(pes[13] >> 1);
    Pts90k::new(value)
}

pub(crate) fn normalise_pts(pts_ms: i64, origin_ms: i64) -> i64 {
    let mut relative = pts_ms - origin_ms;
    if relative < -PTS_WRAP_MS / 2 {
        relative += PTS_WRAP_MS;
    } else if relative > PTS_WRAP_MS / 2 {
        relative -= PTS_WRAP_MS;
    }
    relative
}

pub(crate) fn scan_b24<F, P, C, R>(
    path: &Path,
    track: &B24Track,
    mut on_scene: F,
    mut on_progress: P,
    mut cancelled: C,
    mut on_pes: R,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(u16, native_b24::CaptionScene) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut packet = [0u8; 188];
    let mut pes = Vec::new();
    let mut pes_pid = None;
    let mut last_pts = 0;
    let mut timeline_origin_ms = None;
    let mut summary = B24DecodeSummary::default();
    let mut decoder = native_b24::NativeB24Decoder::new()
        .ok_or_else(|| io::Error::other("could not initialize native B24 decoder"))?;
    let mut next_progress = PROGRESS_INTERVAL;
    let mut pes_offset = 0;
    let mut active_pmt_pid = Some(track.pmt_pid);
    let mut active_caption_pid = None;
    let mut psi = HashMap::<u16, PsiAssembler>::new();

    loop {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "conversion cancelled",
            ));
        }
        match reader.read_exact(&mut packet) {
            Ok(()) => summary.bytes_read += 188,
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        }
        if summary.bytes_read >= next_progress {
            on_progress(&summary);
            next_progress += PROGRESS_INTERVAL;
        }
        let packet_payload = ts_payload(&packet);
        if timeline_origin_ms.is_none()
            && let Some((_, true, payload)) = packet_payload
        {
            timeline_origin_ms = pes_pts_from_header(payload).map(Pts90k::to_millis);
        }
        let Some((pid, payload_start, payload)) = packet_payload else {
            continue;
        };

        if pid == 0 || Some(pid) == active_pmt_pid {
            for section in psi.entry(pid).or_default().push_all(payload, payload_start) {
                if pid == 0 && section.first() == Some(&0x00) {
                    if let Some((_, pmt_pid)) = pmt_programs(&section)
                        .into_iter()
                        .find(|(service_id, _)| *service_id == track.service_id)
                    {
                        active_pmt_pid = Some(pmt_pid);
                    }
                    continue;
                }
                if Some(pid) != active_pmt_pid
                    || section.first() != Some(&0x02)
                    || section.get(3..5) != Some(track.service_id.to_be_bytes().as_slice())
                    || !section.get(5).is_some_and(|flags| flags & 0x01 != 0)
                {
                    continue;
                }
                let next_caption_pid = b24_caption_streams(&section)
                    .into_iter()
                    .find(|stream| stream.component_tag == track.component_tag)
                    .map(|stream| stream.pid);
                if next_caption_pid != active_caption_pid {
                    if let Some(previous_pid) = pes_pid.take() {
                        emit_b24_pes(
                            previous_pid,
                            pes_offset,
                            &mut pes,
                            &mut decoder,
                            &mut last_pts,
                            &mut timeline_origin_ms,
                            &mut summary,
                            &mut on_scene,
                            &mut on_pes,
                        )?;
                    }
                    active_caption_pid = next_caption_pid;
                }
            }
        }

        if Some(pid) != active_caption_pid {
            continue;
        }
        if payload_start && let Some(previous_pid) = pes_pid.take() {
            emit_b24_pes(
                previous_pid,
                pes_offset,
                &mut pes,
                &mut decoder,
                &mut last_pts,
                &mut timeline_origin_ms,
                &mut summary,
                &mut on_scene,
                &mut on_pes,
            )?;
        }
        if pes.len() + payload.len() > PES_BUFFER_LIMIT {
            pes.clear();
            pes_pid = None;
            continue;
        }
        if pes.is_empty() {
            pes_offset = summary.bytes_read.saturating_sub(188);
            pes_pid = Some(pid);
        }
        pes.extend_from_slice(payload);
    }
    if let Some(previous_pid) = pes_pid {
        emit_b24_pes(
            previous_pid,
            pes_offset,
            &mut pes,
            &mut decoder,
            &mut last_pts,
            &mut timeline_origin_ms,
            &mut summary,
            &mut on_scene,
            &mut on_pes,
        )?;
    }
    Ok(summary)
}

#[allow(clippy::too_many_arguments)]
fn emit_b24_pes<F, R>(
    pid: u16,
    pes_offset: u64,
    pes: &mut Vec<u8>,
    decoder: &mut native_b24::NativeB24Decoder,
    last_pts: &mut i64,
    timeline_origin_ms: &mut Option<i64>,
    summary: &mut B24DecodeSummary,
    on_scene: &mut F,
    on_pes: &mut R,
) -> io::Result<()>
where
    F: FnMut(u16, native_b24::CaptionScene) -> io::Result<()>,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    if pes.is_empty() {
        return Ok(());
    }
    on_pes(pid, pes_offset, pes)?;
    if let Some(scene) = flush_b24_pes(pes, decoder, last_pts, timeline_origin_ms, summary) {
        on_scene(pid, scene)?;
    }
    Ok(())
}

pub(crate) fn decode_b24_with_progress<F>(
    path: &Path,
    track: &B24Track,
    mut on_progress: F,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(&B24DecodeSummary),
{
    scan_b24(
        path,
        track,
        |_, _| Ok(()),
        |summary| on_progress(summary),
        || false,
        |_, _, _| Ok(()),
    )
}

#[cfg(test)]
pub(crate) fn decode_b24(path: &Path, track: &B24Track) -> io::Result<B24DecodeSummary> {
    decode_b24_with_progress(path, track, |_| {})
}
