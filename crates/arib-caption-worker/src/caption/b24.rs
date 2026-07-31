use crate::*;

pub(crate) fn b24_payload_from_pes(pes: &[u8]) -> Option<(&[u8], Option<i64>)> {
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

pub(crate) fn pes_pts_from_header(pes: &[u8]) -> Option<i64> {
    if pes.len() < 14 || pes[..3] != [0, 0, 1] || pes[7] & 0x80 == 0 {
        return None;
    }
    let value = (i64::from(pes[9] & 0x0e) << 29)
        | (i64::from(pes[10]) << 22)
        | (i64::from(pes[11] & 0xfe) << 14)
        | (i64::from(pes[12]) << 7)
        | i64::from(pes[13] >> 1);
    Some(value / 90)
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
    F: FnMut(native_b24::CaptionScene) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    let mut reader = BufReader::with_capacity(1024 * 1024, File::open(path)?);
    let mut packet = [0u8; 188];
    let mut pes = Vec::new();
    let mut last_pts = 0;
    let mut timeline_origin_ms = None;
    let mut summary = B24DecodeSummary::default();
    let mut decoder = native_b24::NativeB24Decoder::new()
        .ok_or_else(|| io::Error::other("could not initialize native B24 decoder"))?;
    let mut next_progress = PROGRESS_INTERVAL;
    let mut pes_offset = 0;

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
            timeline_origin_ms = pes_pts_from_header(payload);
        }
        let Some((pid, payload_start, payload)) = packet_payload else {
            continue;
        };
        if pid != track.caption_pid {
            continue;
        }
        if payload_start {
            if !pes.is_empty() {
                on_pes(track.caption_pid, pes_offset, &pes)?;
            }
            if let Some(scene) = flush_b24_pes(
                &mut pes,
                &mut decoder,
                &mut last_pts,
                &mut timeline_origin_ms,
                &mut summary,
            ) {
                on_scene(scene)?;
            }
        }
        if pes.len() + payload.len() > PES_BUFFER_LIMIT {
            pes.clear();
            continue;
        }
        if pes.is_empty() {
            pes_offset = summary.bytes_read.saturating_sub(188);
        }
        pes.extend_from_slice(payload);
    }
    if !pes.is_empty() {
        on_pes(track.caption_pid, pes_offset, &pes)?;
    }
    if let Some(scene) = flush_b24_pes(
        &mut pes,
        &mut decoder,
        &mut last_pts,
        &mut timeline_origin_ms,
        &mut summary,
    ) {
        on_scene(scene)?;
    }
    Ok(summary)
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
        |_| Ok(()),
        |summary| on_progress(summary),
        || false,
        |_, _, _| Ok(()),
    )
}

#[cfg(test)]
pub(crate) fn decode_b24(path: &Path, track: &B24Track) -> io::Result<B24DecodeSummary> {
    decode_b24_with_progress(path, track, |_| {})
}
