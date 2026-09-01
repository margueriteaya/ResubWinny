use serde::Serialize;

use crate::caption::ruby::TtmlRubyBinding;

use crate::resource::TtmlResourceMetadata;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InputKind {
    MpegTs,
    M2ts,
    Tlv,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct InputProbe {
    pub(crate) kind: InputKind,
    pub(crate) packet_size: Option<usize>,
    pub(crate) sync_offset: Option<usize>,
    pub(crate) confidence: usize,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub(crate) struct B24Track {
    pub(crate) service_id: u16,
    pub(crate) pmt_pid: u16,
    pub(crate) caption_pid: u16,
    pub(crate) component_tag: u8,
    pub(crate) caption_pids: Vec<u16>,
    pub(crate) language: Option<String>,
    pub(crate) service_name: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct DataTracks {
    pub(crate) pmt_pid: u16,
    pub(crate) pids: Vec<u16>,
    pub(crate) caption_pids: Vec<u16>,
    pub(crate) superimpose_pids: Vec<u16>,
}

impl DataTracks {
    pub(crate) fn retain_default_caption_tracks(&mut self) {
        if !self.caption_pids.is_empty() {
            self.pids.retain(|pid| self.caption_pids.contains(pid));
        }
    }

    pub(crate) fn component_kind(&self, pid: u16) -> &'static str {
        if self.caption_pids.contains(&pid) {
            "caption"
        } else if self.superimpose_pids.contains(&pid) {
            "superimpose"
        } else {
            "candidate"
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct CaptionTrackInspection {
    pub(crate) label: String,
    pub(crate) detail: String,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct InputInspection {
    pub(crate) bytes: u64,
    pub(crate) container: String,
    pub(crate) route_code: &'static str,
    pub(crate) route: String,
    pub(crate) service: String,
    pub(crate) tracks: Vec<CaptionTrackInspection>,
    pub(crate) broadcast: BroadcastMetadata,
}

#[derive(Debug, Default, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct BroadcastMetadata {
    pub(crate) network_name: Option<String>,
    pub(crate) programme_name: Option<String>,
    pub(crate) programme_description: Option<String>,
    pub(crate) broadcast_time_utc: Option<String>,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct B24DecodeSummary {
    pub bytes_read: u64,
    pub pes_packets: u64,
    pub captions: u64,
    pub regions: u64,
    pub characters: u64,
    pub drcs_glyphs: u64,
    pub decoder_errors: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlCaption {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
    pub(crate) text: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: Option<i32>,
    pub(crate) height: Option<i32>,
    pub(crate) style: TtmlCaptionStyle,
    pub(crate) rich_body: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) ruby_bindings: Vec<TtmlRubyBinding>,
    pub(crate) source: Option<TtmlCaptionSource>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub(crate) struct TtmlCaptionStyle {
    pub(crate) color: Option<String>,
    pub(crate) background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) background_scope: Option<TtmlBackgroundScope>,
    pub(crate) font_size: Option<String>,
    pub(crate) font_family: Option<String>,
    pub(crate) font_style: Option<String>,
    pub(crate) font_weight: Option<String>,
    pub(crate) writing_mode: Option<String>,
    pub(crate) direction: Option<String>,
    pub(crate) text_align: Option<String>,
    pub(crate) text_outline: Option<String>,
    pub(crate) line_height: Option<String>,
    pub(crate) letter_spacing: Option<String>,
    pub(crate) opacity: Option<String>,
    pub(crate) display_align: Option<String>,
    pub(crate) background_image: Option<String>,
    pub(crate) font_resource: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TtmlBackgroundScope {
    Region,
    Block,
    Inline,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub(crate) struct RationalTimestamp {
    pub(crate) value: i64,
    pub(crate) timescale: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // Exactly one route is compiled into a worker backend.
pub(crate) enum TlvTimelineBasis {
    MptPresentationNtp,
    LibaribTlvNormalizedPts,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlCaptionSource {
    pub(crate) route: &'static str,
    pub(crate) source_offset: u64,
    pub(crate) mmpt_packet_id: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mpu_sequence_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mmtp_sequence_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) presentation_ntp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) normalized_pts: Option<RationalTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_pts: Option<RationalTimestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_ntp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reference_start_time_leap_indicator: Option<u8>,
    pub(crate) timeline_basis: TlvTimelineBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) track_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) component_tag: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timing_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) operation_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) display_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) compression_type: Option<u8>,
    pub(crate) random_access: bool,
    pub(crate) discontinuity: bool,
    pub(crate) discontinuity_reasons: u32,
    pub(crate) xml_encoding: String,
    pub(crate) resources: Vec<TtmlResourceMetadata>,
    pub(crate) resources_complete: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct TtmlResourceScope {
    pub(crate) packet_id: u16,
    pub(crate) mpu_sequence_number: u32,
}

impl TtmlResourceScope {
    pub(crate) fn key(&self) -> String {
        format!("packet:{}:mpu:{}", self.packet_id, self.mpu_sequence_number)
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceAssociation {
    pub(crate) status: &'static str,
    pub(crate) scope: Option<TtmlResourceScope>,
    pub(crate) scope_key: Option<String>,
    pub(crate) resource_record_key: Option<String>,
    pub(crate) resource_data_type: Option<u8>,
    pub(crate) resource_byte_length: Option<usize>,
    pub(crate) resource_format_hint: Option<&'static str>,
    pub(crate) resource_format_validation: Option<&'static str>,
    pub(crate) resource_width: Option<u32>,
    pub(crate) resource_height: Option<u32>,
    pub(crate) resource_preview_available: Option<bool>,
    pub(crate) reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceReference {
    pub(crate) uri: String,
    pub(crate) resource_index: u32,
    pub(crate) usage: &'static str,
    pub(crate) extraction: String,
    pub(crate) association: TtmlResourceAssociation,
    pub(crate) source: Option<TtmlCaptionSource>,
}

pub(crate) fn subt_resource_index(value: &str) -> Option<u32> {
    let index = value.strip_prefix("subt://")?;
    (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| index.parse::<u32>().ok())
        .flatten()
}

pub(crate) fn ttml_resource_references(caption: &TtmlCaption) -> Vec<TtmlResourceReference> {
    let scope = caption.source.as_ref().and_then(|source| {
        source
            .mpu_sequence_number
            .map(|mpu_sequence_number| TtmlResourceScope {
                packet_id: source.mmpt_packet_id,
                mpu_sequence_number,
            })
    });
    let scope_key = scope.as_ref().map(TtmlResourceScope::key);
    [
        (caption.style.background_image.as_deref(), "background-image"),
        (caption.style.font_resource.as_deref(), "font-face"),
    ]
    .into_iter()
    .filter_map(|(uri, usage)| {
        let uri = uri?;
        let resource_index = subt_resource_index(uri)?;
        let metadata = u8::try_from(resource_index).ok().and_then(|index| {
            caption
                .source
                .as_ref()
                .and_then(|source| source.resources.iter().find(|item| item.index == index))
        });
        let evidence_match = metadata.is_some();
        let resource_record_key = scope_key.as_ref().zip(metadata).map(|(scope_key, metadata)| {
            format!("stpp-resource:{}:subsample:{}", scope_key, metadata.index)
        });
        Some(TtmlResourceReference {
            uri: uri.to_owned(),
            resource_index,
            usage,
            extraction: if evidence_match {
                "same-MPU resource evidence retained; semantic decode is not enabled".into()
            } else {
                "unresolved: archive reference only; no matching same-MPU resource evidence".into()
            },
            association: TtmlResourceAssociation {
                status: if evidence_match { "same-mpu-evidence" } else { "unresolved" },
                scope: scope.clone(),
                scope_key: scope_key.clone(),
                resource_record_key,
                resource_data_type: metadata.map(|item| item.data_type),
                resource_byte_length: metadata.map(|item| item.byte_length),
                resource_format_hint: metadata.and_then(|item| item.format_hint),
                resource_format_validation: metadata.map(|item| item.format_validation),
                resource_width: metadata.and_then(|item| item.width),
                resource_height: metadata.and_then(|item| item.height),
                resource_preview_available: metadata.map(|item| item.preview_available),
                reason: if evidence_match {
                    "same-MPU resource evidence matched by subsample index; resource bytes remain raw"
                } else if caption.source.as_ref().map(|source| source.resources_complete).unwrap_or(false) {
                    "resource map was complete but the referenced subsample index was absent"
                } else {
                    "subt:// indices are scoped to the current MPU resource map; resource evidence is incomplete"
                },
            },
            source: caption.source.clone(),
        })
    })
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XmlTextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    ShiftJis,
    EucJp,
    Iso2022Jp,
}

impl XmlTextEncoding {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::ShiftJis => "Shift_JIS",
            Self::EucJp => "EUC-JP",
            Self::Iso2022Jp => "ISO-2022-JP",
        }
    }

    pub(crate) fn closing_tag(self) -> &'static [u8] {
        match self {
            Self::Utf16Le => b"<\0/\0t\0t\0>\0",
            Self::Utf16Be => b"\0<\0/\0t\0t\0>",
            _ => b"</tt>",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DecodedTtmlDocument {
    pub(crate) xml: String,
    pub(crate) encoding: XmlTextEncoding,
}
