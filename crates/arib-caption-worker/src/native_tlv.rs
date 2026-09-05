//! Optional, owned Rust boundary for libaribtlv's ARIB STD-B62 events.
//!
//! The upstream C API exposes callback-lifetime views. Every string and byte
//! slice is copied here before the callback returns; no upstream pointer is
//! retained by the worker.

use crate::*;
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{CStr, c_char, c_void},
    io::{self, Read},
    marker::PhantomData,
    path::Path,
    ptr::NonNull,
    rc::Rc,
    slice,
};

const BRIDGE_ABI_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTlvSubtitleTrack {
    pub(crate) track_id: u64,
    pub(crate) context_id: u32,
    pub(crate) packet_id: u16,
    pub(crate) component_tag: u16,
    pub(crate) language: Option<String>,
    pub(crate) tag: u8,
    pub(crate) info_version: u8,
    pub(crate) subtitle_type: u8,
    pub(crate) format: u8,
    pub(crate) operation_mode: u8,
    pub(crate) timing_mode: u8,
    pub(crate) display_mode: u8,
    pub(crate) resolution: u8,
    pub(crate) compression_type: u8,
    pub(crate) start_mpu_sequence_number: Option<u32>,
    pub(crate) reference_start_ntp: Option<u64>,
    pub(crate) reference_start_time_leap_indicator: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTlvSubtitleResource {
    pub(crate) subsample_number: u8,
    pub(crate) data_type: u8,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTlvCaptionUnit {
    pub(crate) track_id: u64,
    pub(crate) component_tag: u16,
    pub(crate) bytes: Vec<u8>,
    pub(crate) pts: (i64, u32),
    pub(crate) input_offset: u64,
    pub(crate) random_access: bool,
    pub(crate) discontinuity: bool,
    pub(crate) discontinuity_reasons: u32,
    pub(crate) timing_mode: Option<u8>,
    pub(crate) operation_mode: Option<u8>,
    pub(crate) display_mode: Option<u8>,
    pub(crate) compression_type: Option<u8>,
    pub(crate) mpu_sequence_number: Option<u32>,
    pub(crate) reference_start_pts: Option<(i64, u32)>,
    pub(crate) resources: Vec<NativeTlvSubtitleResource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTlvError {
    pub(crate) code: i32,
    pub(crate) input_offset: u64,
    pub(crate) recoverable: bool,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeTlvEvent {
    Track(NativeTlvSubtitleTrack),
    Caption(NativeTlvCaptionUnit),
    Error(NativeTlvError),
}

#[repr(C)]
struct RawDemuxer {
    _private: [u8; 0],
}

#[repr(C)]
struct RawSubtitleTrack {
    track_id: u64,
    context_id: u32,
    packet_id: u16,
    component_tag: u16,
    tag: u8,
    info_version: u8,
    subtitle_type: u8,
    format: u8,
    operation_mode: u8,
    timing_mode: u8,
    display_mode: u8,
    resolution: u8,
    compression_type: u8,
    has_start_mpu_sequence_number: u8,
    start_mpu_sequence_number: u32,
    has_reference_start_ntp: u8,
    reference_start_ntp: u64,
    reference_start_time_leap_indicator: u8,
    language: *const c_char,
}

#[repr(C)]
struct RawSubtitleResource {
    subsample_number: u8,
    data_type: u8,
    data: *const u8,
    size: usize,
}

#[repr(C)]
struct RawCaptionUnit {
    track_id: u64,
    component_tag: u16,
    data: *const u8,
    size: usize,
    pts_value: i64,
    pts_timescale: u32,
    input_offset: u64,
    random_access: u8,
    discontinuity: u8,
    discontinuity_reasons: u32,
    has_timing_mode: u8,
    timing_mode: u8,
    has_operation_mode: u8,
    operation_mode: u8,
    has_display_mode: u8,
    display_mode: u8,
    has_compression_type: u8,
    compression_type: u8,
    has_mpu_sequence_number: u8,
    mpu_sequence_number: u32,
    has_reference_start_pts: u8,
    reference_start_pts_value: i64,
    reference_start_pts_timescale: u32,
    resources: *const RawSubtitleResource,
    resource_count: usize,
}

#[repr(C)]
struct RawError {
    code: i32,
    input_offset: u64,
    recoverable: u8,
    message: *const c_char,
}

type TrackCallback = unsafe extern "C" fn(*mut c_void, *const RawSubtitleTrack);
type CaptionCallback = unsafe extern "C" fn(*mut c_void, *const RawCaptionUnit);
type ErrorCallback = unsafe extern "C" fn(*mut c_void, *const RawError);

#[repr(C)]
struct RawCallbacks {
    struct_size: usize,
    on_track: Option<TrackCallback>,
    on_caption: Option<CaptionCallback>,
    on_error: Option<ErrorCallback>,
}

unsafe extern "C" {
    fn resub_aribtlv_bridge_abi_version() -> u32;
    fn resub_aribtlv_create(callbacks: *const RawCallbacks, opaque: *mut c_void)
    -> *mut RawDemuxer;
    fn resub_aribtlv_destroy(demuxer: *mut RawDemuxer);
    fn resub_aribtlv_push(demuxer: *mut RawDemuxer, data: *const u8, size: usize) -> i32;
    fn resub_aribtlv_flush(demuxer: *mut RawDemuxer) -> i32;
    fn resub_aribtlv_last_error(demuxer: *const RawDemuxer) -> *const c_char;
}

#[derive(Default)]
struct CallbackState {
    events: VecDeque<NativeTlvEvent>,
}

pub(crate) struct NativeTlvDemuxer {
    inner: NonNull<RawDemuxer>,
    state: Box<CallbackState>,
    // The native demuxer invokes callbacks synchronously and is deliberately
    // confined to its creating thread.
    _not_send: PhantomData<Rc<()>>,
}

impl NativeTlvDemuxer {
    pub(crate) fn new() -> io::Result<Self> {
        // SAFETY: the function takes no pointers and returns a value.
        let version = unsafe { resub_aribtlv_bridge_abi_version() };
        if version != BRIDGE_ABI_VERSION {
            return Err(io::Error::other(format!(
                "unsupported resub aribtlv bridge ABI {version}; expected {BRIDGE_ABI_VERSION}"
            )));
        }
        let mut state = Box::<CallbackState>::default();
        let callbacks = RawCallbacks {
            struct_size: size_of::<RawCallbacks>(),
            on_track: Some(on_track),
            on_caption: Some(on_caption),
            on_error: Some(on_error),
        };
        // SAFETY: `state` is boxed and remains at the same address until after
        // `inner` is destroyed. The bridge copies the callback table.
        let inner = unsafe {
            resub_aribtlv_create(
                &callbacks,
                (&mut *state as *mut CallbackState).cast::<c_void>(),
            )
        };
        let inner = NonNull::new(inner)
            .ok_or_else(|| io::Error::other("could not create the libaribtlv demuxer"))?;
        Ok(Self {
            inner,
            state,
            _not_send: PhantomData,
        })
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> io::Result<()> {
        // SAFETY: the input view remains valid for the synchronous call.
        let result =
            unsafe { resub_aribtlv_push(self.inner.as_ptr(), bytes.as_ptr(), bytes.len()) };
        self.check_result(result)
    }

    pub(crate) fn flush(&mut self) -> io::Result<()> {
        // SAFETY: `inner` is owned by self and still live.
        let result = unsafe { resub_aribtlv_flush(self.inner.as_ptr()) };
        self.check_result(result)
    }

    pub(crate) fn drain(&mut self) -> impl Iterator<Item = NativeTlvEvent> + '_ {
        self.state.events.drain(..)
    }

    fn check_result(&self, result: i32) -> io::Result<()> {
        if result == 0 {
            return Ok(());
        }
        // SAFETY: the returned NUL-terminated view belongs to the live demuxer
        // and is copied before this method returns.
        let message = unsafe { copy_c_string(resub_aribtlv_last_error(self.inner.as_ptr())) }
            .unwrap_or_else(|| format!("libaribtlv failed with result {result}"));
        Err(io::Error::other(message))
    }
}

impl Drop for NativeTlvDemuxer {
    fn drop(&mut self) {
        // SAFETY: this is the unique owned handle and it is destroyed once.
        unsafe { resub_aribtlv_destroy(self.inner.as_ptr()) };
    }
}

pub(crate) fn rational_delta_ms(
    value: RationalTimestamp,
    origin: RationalTimestamp,
) -> Option<i64> {
    if value.timescale == 0 || origin.timescale == 0 {
        return None;
    }
    let numerator = i128::from(value.value)
        .checked_mul(i128::from(origin.timescale))?
        .checked_sub(i128::from(origin.value).checked_mul(i128::from(value.timescale))?)?
        .checked_mul(1_000)?;
    let denominator = i128::from(value.timescale).checked_mul(i128::from(origin.timescale))?;
    i64::try_from(numerator / denominator).ok()
}

fn strict_ttml_document(bytes: &[u8]) -> Option<DecodedTtmlDocument> {
    let (start, detected) = ttml_document_start_bytes(bytes)?;
    let allowed_prefix = match detected {
        XmlTextEncoding::Utf8 => bytes.get(..start) == Some(&[0xef, 0xbb, 0xbf][..]),
        XmlTextEncoding::Utf16Le => bytes.get(..start) == Some(&[0xff, 0xfe][..]),
        XmlTextEncoding::Utf16Be => bytes.get(..start) == Some(&[0xfe, 0xff][..]),
        _ => false,
    };
    if start != 0 && !allowed_prefix {
        return None;
    }
    let candidate = bytes.get(start..)?;
    let encoding = xml_encoding_for_candidate(candidate, detected)?;
    let closing = encoding.closing_tag();
    let end = find_bytes(candidate, closing)?.checked_add(closing.len())?;
    let tail = candidate.get(end..)?;
    let tail_is_whitespace = match encoding {
        XmlTextEncoding::Utf16Le => {
            let mut pairs = tail.chunks_exact(2);
            pairs
                .by_ref()
                .all(|pair| pair[1] == 0 && matches!(pair[0], b' ' | b'\t' | b'\r' | b'\n'))
                && pairs.remainder().is_empty()
        }
        XmlTextEncoding::Utf16Be => {
            let mut pairs = tail.chunks_exact(2);
            pairs
                .by_ref()
                .all(|pair| pair[0] == 0 && matches!(pair[1], b' ' | b'\t' | b'\r' | b'\n'))
                && pairs.remainder().is_empty()
        }
        _ => tail.iter().all(u8::is_ascii_whitespace),
    };
    if !tail_is_whitespace {
        return None;
    }
    let xml = decode_xml_bytes(candidate.get(..end)?, encoding)?;
    is_complete_ttml_document(&xml).then_some(DecodedTtmlDocument { xml, encoding })
}

#[allow(clippy::too_many_arguments)]
fn process_native_event<F, R>(
    event: NativeTlvEvent,
    tracks: &mut BTreeMap<u64, NativeTlvSubtitleTrack>,
    timeline_origin: &mut Option<RationalTimestamp>,
    summary: &mut B24DecodeSummary,
    on_caption: &mut F,
    on_payload: &mut R,
) -> io::Result<()>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    R: FnMut(u64, &TlvCaptionPayload) -> io::Result<()>,
{
    let unit = match event {
        NativeTlvEvent::Track(track) => {
            tracks.insert(track.track_id, track);
            return Ok(());
        }
        NativeTlvEvent::Error(error) => {
            summary.decoder_errors += 1;
            if error.recoverable {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "libaribtlv error {} at byte {}: {}",
                    error.code, error.input_offset, error.message
                ),
            ));
        }
        NativeTlvEvent::Caption(unit) => unit,
    };
    let Some(track) = tracks.get(&unit.track_id) else {
        summary.decoder_errors += 1;
        return Ok(());
    };
    let mpu_sequence_number = unit.mpu_sequence_number;
    let resources: Vec<TlvSubtitleResource> = unit
        .resources
        .iter()
        .map(|resource| TlvSubtitleResource {
            index: resource.subsample_number,
            data_type: resource.data_type,
            bytes: resource.bytes.clone(),
        })
        .collect();
    let payload = TlvCaptionPayload {
        packet_id: track.packet_id,
        mpu_sequence_number,
        mmtp_sequence_number: None,
        presentation_ntp: None,
        timed: None,
        bytes: unit.bytes.clone(),
        resources,
        resources_complete: mpu_sequence_number.is_some(),
    };
    on_payload(unit.input_offset, &payload)?;

    let compression_type = unit.compression_type.unwrap_or(track.compression_type);
    if compression_type != 0 {
        // B62 compression 1/2 is EXI. Preserve the raw AU above, but never
        // submit compressed or unknown bytes to the XML parser.
        summary.decoder_errors += 1;
        return Ok(());
    }
    let Some(document) = strict_ttml_document(&unit.bytes) else {
        summary.decoder_errors += 1;
        return Ok(());
    };
    let pts = RationalTimestamp {
        value: unit.pts.0,
        timescale: unit.pts.1,
    };
    if pts.timescale == 0 {
        summary.decoder_errors += 1;
        return Ok(());
    }
    let origin = *timeline_origin.get_or_insert(pts);
    let Some(base_ms) = rational_delta_ms(pts, origin) else {
        summary.decoder_errors += 1;
        return Ok(());
    };
    summary.pes_packets += 1;
    for mut caption in parse_ttml_captions(&document.xml, base_ms) {
        summary.captions += 1;
        summary.characters += caption.text.chars().count() as u64;
        caption.source = Some(TtmlCaptionSource {
            route: "isdb_s3_tlv_libaribtlv_b62",
            source_offset: unit.input_offset,
            mmpt_packet_id: track.packet_id,
            mpu_sequence_number,
            mmtp_sequence_number: None,
            presentation_ntp: None,
            normalized_pts: Some(pts),
            reference_start_pts: unit
                .reference_start_pts
                .map(|(value, timescale)| RationalTimestamp { value, timescale }),
            reference_start_ntp: track.reference_start_ntp,
            reference_start_time_leap_indicator: track
                .reference_start_ntp
                .map(|_| track.reference_start_time_leap_indicator),
            timeline_basis: TlvTimelineBasis::LibaribTlvNormalizedPts,
            track_id: Some(unit.track_id),
            component_tag: Some(unit.component_tag),
            timing_mode: unit.timing_mode.or(Some(track.timing_mode)),
            operation_mode: unit.operation_mode.or(Some(track.operation_mode)),
            display_mode: unit.display_mode.or(Some(track.display_mode)),
            compression_type: Some(compression_type),
            random_access: unit.random_access,
            discontinuity: unit.discontinuity,
            discontinuity_reasons: unit.discontinuity_reasons,
            xml_encoding: document.encoding.label().to_owned(),
            resources: payload
                .resources
                .iter()
                .map(|resource| {
                    let format = bounded_resource_format(&resource.bytes);
                    TtmlResourceMetadata {
                        index: resource.index,
                        data_type: resource.data_type,
                        byte_length: resource.bytes.len(),
                        format_hint: format.format_hint,
                        format_validation: format.format_validation,
                        width: format.width,
                        height: format.height,
                        preview_available: bounded_png_preview_data_uri(&resource.bytes).is_some(),
                    }
                })
                .collect(),
            resources_complete: payload.resources_complete,
        });
        on_caption(caption)?;
    }
    Ok(())
}

pub(crate) fn scan_tlv_ttml_native<F, P, C, R, A>(
    path: &Path,
    mut on_caption: F,
    mut on_progress: P,
    mut cancelled: C,
    mut on_payload: R,
    _on_asset: A,
) -> io::Result<B24DecodeSummary>
where
    F: FnMut(TtmlCaption) -> io::Result<()>,
    P: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
    R: FnMut(u64, &TlvCaptionPayload) -> io::Result<()>,
    A: FnMut(TlvAssetEvidence) -> io::Result<()>,
{
    let probe = probe_path(path)?;
    if probe.kind != InputKind::Tlv {
        return Err(io::Error::other(
            "TLV TTML conversion requires an ISDB-S3 TLV input",
        ));
    }
    let mut input = crate::input::open_input(path)?;
    let mut demuxer = NativeTlvDemuxer::new()?;
    let mut tracks = BTreeMap::new();
    let mut timeline_origin = None;
    let mut summary = B24DecodeSummary::default();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        if cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "conversion cancelled",
            ));
        }
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        demuxer.push(&buffer[..read])?;
        summary.bytes_read = summary.bytes_read.saturating_add(read as u64);
        for event in demuxer.drain().collect::<Vec<_>>() {
            process_native_event(
                event,
                &mut tracks,
                &mut timeline_origin,
                &mut summary,
                &mut on_caption,
                &mut on_payload,
            )?;
        }
        on_progress(&summary);
    }
    demuxer.flush()?;
    for event in demuxer.drain().collect::<Vec<_>>() {
        process_native_event(
            event,
            &mut tracks,
            &mut timeline_origin,
            &mut summary,
            &mut on_caption,
            &mut on_payload,
        )?;
    }
    if summary.captions == 0 {
        return Err(io::Error::other(
            "no complete normalized XML TTML captions were found in discovered TLV subtitle assets; use dump-tlv for raw evidence",
        ));
    }
    Ok(summary)
}

unsafe extern "C" fn on_track(opaque: *mut c_void, raw: *const RawSubtitleTrack) {
    // SAFETY: both pointers are supplied by the bridge for this callback.
    let (Some(state), Some(raw)) = (unsafe { opaque.cast::<CallbackState>().as_mut() }, unsafe {
        raw.as_ref()
    }) else {
        return;
    };
    state
        .events
        .push_back(NativeTlvEvent::Track(NativeTlvSubtitleTrack {
            track_id: raw.track_id,
            context_id: raw.context_id,
            packet_id: raw.packet_id,
            component_tag: raw.component_tag,
            language: unsafe { copy_c_string(raw.language) },
            tag: raw.tag,
            info_version: raw.info_version,
            subtitle_type: raw.subtitle_type,
            format: raw.format,
            operation_mode: raw.operation_mode,
            timing_mode: raw.timing_mode,
            display_mode: raw.display_mode,
            resolution: raw.resolution,
            compression_type: raw.compression_type,
            start_mpu_sequence_number: (raw.has_start_mpu_sequence_number != 0)
                .then_some(raw.start_mpu_sequence_number),
            reference_start_ntp: (raw.has_reference_start_ntp != 0)
                .then_some(raw.reference_start_ntp),
            reference_start_time_leap_indicator: raw.reference_start_time_leap_indicator,
        }));
}

unsafe extern "C" fn on_caption(opaque: *mut c_void, raw: *const RawCaptionUnit) {
    // SAFETY: both pointers are supplied by the bridge for this callback.
    let (Some(state), Some(raw)) = (unsafe { opaque.cast::<CallbackState>().as_mut() }, unsafe {
        raw.as_ref()
    }) else {
        return;
    };
    let resources = unsafe { copy_raw_slice(raw.resources, raw.resource_count) }
        .iter()
        .map(|resource| NativeTlvSubtitleResource {
            subsample_number: resource.subsample_number,
            data_type: resource.data_type,
            bytes: unsafe { copy_raw_slice(resource.data, resource.size) }.to_vec(),
        })
        .collect();
    state
        .events
        .push_back(NativeTlvEvent::Caption(NativeTlvCaptionUnit {
            track_id: raw.track_id,
            component_tag: raw.component_tag,
            bytes: unsafe { copy_raw_slice(raw.data, raw.size) }.to_vec(),
            pts: (raw.pts_value, raw.pts_timescale),
            input_offset: raw.input_offset,
            random_access: raw.random_access != 0,
            discontinuity: raw.discontinuity != 0,
            discontinuity_reasons: raw.discontinuity_reasons,
            timing_mode: (raw.has_timing_mode != 0).then_some(raw.timing_mode),
            operation_mode: (raw.has_operation_mode != 0).then_some(raw.operation_mode),
            display_mode: (raw.has_display_mode != 0).then_some(raw.display_mode),
            compression_type: (raw.has_compression_type != 0).then_some(raw.compression_type),
            mpu_sequence_number: (raw.has_mpu_sequence_number != 0)
                .then_some(raw.mpu_sequence_number),
            reference_start_pts: (raw.has_reference_start_pts != 0).then_some((
                raw.reference_start_pts_value,
                raw.reference_start_pts_timescale,
            )),
            resources,
        }));
}

unsafe extern "C" fn on_error(opaque: *mut c_void, raw: *const RawError) {
    // SAFETY: both pointers are supplied by the bridge for this callback.
    let (Some(state), Some(raw)) = (unsafe { opaque.cast::<CallbackState>().as_mut() }, unsafe {
        raw.as_ref()
    }) else {
        return;
    };
    state
        .events
        .push_back(NativeTlvEvent::Error(NativeTlvError {
            code: raw.code,
            input_offset: raw.input_offset,
            recoverable: raw.recoverable != 0,
            message: unsafe { copy_c_string(raw.message) }.unwrap_or_default(),
        }));
}

unsafe fn copy_raw_slice<'a, T>(pointer: *const T, length: usize) -> &'a [T] {
    if length == 0 || pointer.is_null() {
        return &[];
    }
    // SAFETY: callers use callback-lifetime views with the exact upstream
    // length, and consume the returned slice before the callback returns.
    unsafe { slice::from_raw_parts(pointer, length) }
}

unsafe fn copy_c_string(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: bridge strings are documented as NUL-terminated callback views.
    Some(
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_constructs_an_owned_demuxer() {
        let mut demuxer = NativeTlvDemuxer::new().expect("libaribtlv demuxer");
        demuxer.push(&[]).expect("empty bounded input");
        demuxer.flush().expect("flush");
        assert_eq!(demuxer.drain().count(), 0);
    }

    #[test]
    fn caption_callback_copies_document_and_resources_before_returning() {
        let mut state = CallbackState::default();
        let mut document = b"<tt/>".to_vec();
        let mut image = [0x89, b'P', b'N', b'G'];
        let resources = [RawSubtitleResource {
            subsample_number: 1,
            data_type: 2,
            data: image.as_ptr(),
            size: image.len(),
        }];
        let raw = RawCaptionUnit {
            track_id: 9,
            component_tag: 4,
            data: document.as_ptr(),
            size: document.len(),
            pts_value: 90_000,
            pts_timescale: 90_000,
            input_offset: 123,
            random_access: 1,
            discontinuity: 0,
            discontinuity_reasons: 0,
            has_timing_mode: 1,
            timing_mode: 2,
            has_operation_mode: 1,
            operation_mode: 0,
            has_display_mode: 1,
            display_mode: 1,
            has_compression_type: 1,
            compression_type: 0,
            has_mpu_sequence_number: 1,
            mpu_sequence_number: 77,
            has_reference_start_pts: 1,
            reference_start_pts_value: 45_000,
            reference_start_pts_timescale: 90_000,
            resources: resources.as_ptr(),
            resource_count: resources.len(),
        };
        // SAFETY: all callback views above are valid for this call.
        unsafe { on_caption((&mut state as *mut CallbackState).cast::<c_void>(), &raw) };
        document.fill(0);
        image.fill(0);

        let NativeTlvEvent::Caption(caption) = state.events.pop_front().expect("caption") else {
            panic!("expected a caption event");
        };
        assert_eq!(caption.bytes, b"<tt/>");
        assert_eq!(caption.resources[0].bytes, [0x89, b'P', b'N', b'G']);
        assert_eq!(caption.mpu_sequence_number, Some(77));
        assert_eq!(caption.reference_start_pts, Some((45_000, 90_000)));
    }

    #[test]
    fn rational_timestamps_keep_precision_and_reject_zero_timescales() {
        let origin = RationalTimestamp {
            value: 90_000,
            timescale: 90_000,
        };
        assert_eq!(
            rational_delta_ms(
                RationalTimestamp {
                    value: 135_000,
                    timescale: 90_000,
                },
                origin,
            ),
            Some(500)
        );
        assert_eq!(
            rational_delta_ms(
                RationalTimestamp {
                    value: 1,
                    timescale: 0,
                },
                origin,
            ),
            None
        );
    }

    #[test]
    fn missing_mpu_scope_keeps_raw_caption_evidence() {
        let mut tracks = BTreeMap::from([(
            12,
            NativeTlvSubtitleTrack {
                track_id: 12,
                context_id: 1,
                packet_id: 0x345,
                component_tag: 2,
                language: Some("jpn".to_owned()),
                tag: 0,
                info_version: 1,
                subtitle_type: 0,
                format: 0,
                operation_mode: 0,
                timing_mode: 0,
                display_mode: 0,
                resolution: 0,
                compression_type: 1,
                start_mpu_sequence_number: None,
                reference_start_ntp: None,
                reference_start_time_leap_indicator: 0,
            },
        )]);
        let unit = NativeTlvCaptionUnit {
            track_id: 12,
            component_tag: 2,
            bytes: vec![0xde, 0xad, 0xbe, 0xef],
            pts: (1, 1),
            input_offset: 99,
            random_access: false,
            discontinuity: false,
            discontinuity_reasons: 0,
            timing_mode: None,
            operation_mode: None,
            display_mode: None,
            compression_type: Some(1),
            mpu_sequence_number: None,
            reference_start_pts: None,
            resources: vec![NativeTlvSubtitleResource {
                subsample_number: 1,
                data_type: 2,
                bytes: vec![1, 2, 3],
            }],
        };
        let mut origin = None;
        let mut summary = B24DecodeSummary::default();
        let mut captions = Vec::new();
        let mut payloads = Vec::new();
        process_native_event(
            NativeTlvEvent::Caption(unit),
            &mut tracks,
            &mut origin,
            &mut summary,
            &mut |caption| {
                captions.push(caption);
                Ok(())
            },
            &mut |offset, payload| {
                payloads.push((offset, payload.clone()));
                Ok(())
            },
        )
        .expect("preserve raw evidence");

        assert!(captions.is_empty());
        assert_eq!(summary.decoder_errors, 1);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, 99);
        assert_eq!(payloads[0].1.mpu_sequence_number, None);
        assert!(!payloads[0].1.resources_complete);
        assert_eq!(payloads[0].1.bytes, [0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn strict_native_document_rejects_framing_and_multiple_documents() {
        assert!(strict_ttml_document(b"<tt><body/></tt>").is_some());
        assert!(strict_ttml_document(b"\xef\xbb\xbf<tt><body/></tt>\n").is_some());
        assert!(strict_ttml_document(b"prefix<tt><body/></tt>").is_none());
        assert!(strict_ttml_document(b"<tt></tt><tt></tt>").is_none());
        assert!(strict_ttml_document(b"<tt></tt>garbage").is_none());

        let mut utf16_le = vec![0xff, 0xfe];
        utf16_le.extend("<tt><body/></tt>".encode_utf16().flat_map(u16::to_le_bytes));
        assert!(strict_ttml_document(&utf16_le).is_some());
        utf16_le.push(b' ');
        assert!(strict_ttml_document(&utf16_le).is_none());
    }
}
