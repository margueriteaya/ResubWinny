use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use crate::{
    B24DecodeSummary, DataTracks, PSI_SCAN_BYTES, PsiAssembler, TtmlCaption,
    caption::ttml::{TsFraming, scan_ts_ttml_impl},
    data_pids, first_pmt_pid, ts_payload,
};

/// Discover private data PIDs in a 192-byte recorder packet stream.
///
/// M2TS is a recorder packetisation of MPEG-2 TS. The four-byte prefix is
/// deliberately discarded only at the packet boundary; all PSI/PES parsing
/// remains in the shared bounded MPEG-TS helpers.
pub(crate) fn discover_m2ts_data_tracks(path: &Path) -> io::Result<Option<DataTracks>> {
    let mut file = File::open(path)?;
    let mut bytes = vec![0; PSI_SCAN_BYTES];
    let length = file.read(&mut bytes)?;
    let mut pmt_pid = None;
    let mut psi = std::collections::HashMap::<u16, PsiAssembler>::new();
    for packet in bytes[..length].chunks_exact(192) {
        let Some((pid, start, payload)) = ts_payload(&packet[4..]) else {
            continue;
        };
        if pid != 0 && Some(pid) != pmt_pid {
            continue;
        }
        let Some(section) = psi.entry(pid).or_default().push(payload, start) else {
            continue;
        };
        if pid == 0 && section[0] == 0x00 {
            pmt_pid = first_pmt_pid(&section);
        }
        if Some(pid) == pmt_pid && section[0] == 0x02 {
            let pids = data_pids(&section);
            if !pids.is_empty() {
                return Ok(Some(DataTracks { pmt_pid: pid, pids }));
            }
        }
    }
    Ok(None)
}

/// M2TS owns packetisation and route selection; TTML owns document semantics.
/// This façade keeps callers independent from the caption parser's internals.
pub(crate) fn scan_m2ts_ttml<F, P, C, R>(
    path: &Path,
    tracks: &DataTracks,
    on_caption: F,
    on_progress: P,
    cancelled: C,
    on_pes: R,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u16, u64, &[u8]) -> io::Result<()>,
{
    scan_ts_ttml_impl(
        path,
        tracks,
        TsFraming {
            packet_size: 192,
            ts_offset: 4,
        },
        on_caption,
        on_progress,
        cancelled,
        on_pes,
    )
}
