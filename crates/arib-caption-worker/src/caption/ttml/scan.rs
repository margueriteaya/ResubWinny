use crate::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TsFraming {
    pub(crate) packet_size: usize,
    pub(crate) ts_offset: usize,
}

const M2TS_ATS_WRAP: i64 = 1_i64 << 30;
const M2TS_ATS_HALF_WRAP: i64 = M2TS_ATS_WRAP / 2;

#[derive(Default)]
struct M2tsArrivalClock {
    last_raw: Option<i64>,
    unwrapped: i64,
    origin: Option<i64>,
}

impl M2tsArrivalClock {
    fn update(&mut self, packet: &[u8], framing: TsFraming) -> Option<i64> {
        if framing.packet_size != 192 || framing.ts_offset != 4 || packet.len() < 4 {
            return None;
        }
        let raw = (i64::from(packet[0] & 0x3f) << 24)
            | (i64::from(packet[1]) << 16)
            | (i64::from(packet[2]) << 8)
            | i64::from(packet[3]);
        if let Some(previous) = self.last_raw {
            let mut delta = raw - previous;
            if delta < -M2TS_ATS_HALF_WRAP {
                delta += M2TS_ATS_WRAP;
            } else if delta > M2TS_ATS_HALF_WRAP {
                delta -= M2TS_ATS_WRAP;
            }
            self.unwrapped = self.unwrapped.saturating_add(delta);
        } else {
            self.unwrapped = raw;
        }
        self.last_raw = Some(raw);
        let origin = *self.origin.get_or_insert(self.unwrapped);
        Some(self.unwrapped.saturating_sub(origin) / 27_000)
    }
}

struct PendingTtmlDocument {
    xml: String,
    start_ms: i64,
}

fn emit_pending_documents<F>(
    pending: Vec<PendingTtmlDocument>,
    end_ms: i64,
    summary: &mut B24DecodeSummary,
    on_caption: &mut F,
) -> io::Result<()>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
{
    for document in pending {
        for caption in parse_ttml_captions_until(&document.xml, document.start_ms, Some(end_ms)) {
            summary.captions += 1;
            summary.characters += caption.text.chars().count() as u64;
            summary.features.observe_ttml(&caption);
            on_caption(caption)?;
        }
    }
    Ok(())
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
    let mut reader = BufReader::with_capacity(1024 * 1024, crate::input::open_input(path)?);
    let mut packet = vec![0u8; framing.packet_size];
    let mut pes: HashMap<u16, Vec<u8>> = tracks.pids.iter().map(|pid| (*pid, Vec::new())).collect();
    let mut timeline_origin_ms = None;
    let mut pes_offsets: HashMap<u16, u64> = HashMap::new();
    let mut pes_times: HashMap<u16, i64> = HashMap::new();
    let mut pending_documents: HashMap<u16, Vec<PendingTtmlDocument>> = HashMap::new();
    let mut arrival_clock = M2tsArrivalClock::default();
    let mut last_transport_time_ms = 0_i64;
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
        if let Some(time_ms) = arrival_clock.update(&packet, framing) {
            last_transport_time_ms = time_ms;
        }
        let Some((pid, payload_start, payload)) = ts_payload(&packet[framing.ts_offset..]) else {
            continue;
        };
        let Some(buffer) = pes.get_mut(&pid) else {
            continue;
        };
        if payload_start && !buffer.is_empty() {
            on_pes(
                pid,
                *pes_offsets.get(&pid).unwrap_or(&summary.bytes_read),
                buffer,
            )?;
            let transport_time = *pes_times.get(&pid).unwrap_or(&last_transport_time_ms);
            let pts = if framing.packet_size == 192 {
                transport_time
            } else if let Some(value) = pes_pts_from_header(buffer) {
                let value = value.to_millis();
                let origin = *timeline_origin_ms.get_or_insert(value);
                normalise_pts(value, origin)
            } else {
                transport_time
            };
            let documents = ttml_documents(buffer);
            if !documents.is_empty() {
                if let Some(pending) = pending_documents.remove(&pid) {
                    emit_pending_documents(pending, pts, &mut summary, &mut on_caption)?;
                }
                let pending = documents
                    .into_iter()
                    .filter(|document| ttml_document_has_paragraph(&document.xml))
                    .map(|document| PendingTtmlDocument {
                        xml: document.xml,
                        start_ms: pts,
                    })
                    .collect::<Vec<_>>();
                if !pending.is_empty() {
                    pending_documents.insert(pid, pending);
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
            pes_times.insert(pid, last_transport_time_ms);
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
        let transport_time = *pes_times.get(&pid).unwrap_or(&last_transport_time_ms);
        let pts = if framing.packet_size == 192 {
            transport_time
        } else if let Some(value) = pes_pts_from_header(&buffer) {
            let value = value.to_millis();
            let origin = *timeline_origin_ms.get_or_insert(value);
            normalise_pts(value, origin)
        } else {
            transport_time
        };
        let documents = ttml_documents(&buffer);
        if !documents.is_empty() {
            if let Some(pending) = pending_documents.remove(&pid) {
                emit_pending_documents(pending, pts, &mut summary, &mut on_caption)?;
            }
            let pending = documents
                .into_iter()
                .filter(|document| ttml_document_has_paragraph(&document.xml))
                .map(|document| PendingTtmlDocument {
                    xml: document.xml,
                    start_ms: pts,
                })
                .collect::<Vec<_>>();
            if !pending.is_empty() {
                pending_documents.insert(pid, pending);
            }
        }
        summary.pes_packets += 1;
    }
    for pending in pending_documents.into_values() {
        emit_pending_documents(
            pending,
            last_transport_time_ms,
            &mut summary,
            &mut on_caption,
        )?;
    }
    Ok(summary)
}
