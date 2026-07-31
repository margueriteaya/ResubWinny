use base64::Engine;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceMetadata {
    pub(crate) index: u8,
    pub(crate) data_type: u8,
    pub(crate) byte_length: usize,
    pub(crate) format_hint: Option<&'static str>,
    pub(crate) format_validation: &'static str,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) preview_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TlvSubtitleResource {
    pub(crate) index: u8,
    pub(crate) data_type: u8,
    pub(crate) bytes: Vec<u8>,
}

/// A bounded, lossless archive record for one resource belonging to an STPP
/// MPU. The scope key is intentionally the same key used by `subt://`
/// references; it is never a global asset identifier.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TtmlResourceEvidence {
    pub(crate) record_key: String,
    pub(crate) scope_key: String,
    pub(crate) mmpt_packet_id: u16,
    pub(crate) mpu_sequence_number: u32,
    pub(crate) subsample_number: u8,
    pub(crate) data_type: u8,
    pub(crate) byte_length: usize,
    pub(crate) format_hint: Option<&'static str>,
    pub(crate) format_validation: &'static str,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) preview_data_uri: Option<String>,
    pub(crate) payload_base64: String,
}

pub(crate) fn ttml_resource_evidence(
    packet_id: u16,
    mpu_sequence_number: u32,
    resource: &TlvSubtitleResource,
) -> TtmlResourceEvidence {
    let format = bounded_resource_format(&resource.bytes);
    let scope_key = format!("packet:{packet_id}:mpu:{mpu_sequence_number}");
    TtmlResourceEvidence {
        record_key: format!("stpp-resource:{scope_key}:subsample:{}", resource.index),
        scope_key,
        mmpt_packet_id: packet_id,
        mpu_sequence_number,
        subsample_number: resource.index,
        data_type: resource.data_type,
        byte_length: resource.bytes.len(),
        format_hint: format.format_hint,
        format_validation: format.format_validation,
        width: format.width,
        height: format.height,
        preview_data_uri: bounded_png_preview_data_uri(&resource.bytes),
        payload_base64: base64::engine::general_purpose::STANDARD.encode(&resource.bytes),
    }
}

#[derive(Debug, Default)]
pub(crate) struct TlvSubtitleResourceState {
    pub(crate) last_subsample_number: u8,
    pub(crate) resources: BTreeMap<u8, TlvSubtitleResource>,
    pub(crate) total_bytes: usize,
    pub(crate) overflowed: bool,
}

impl TlvSubtitleResourceState {
    pub(crate) fn add(&mut self, resource: TlvSubtitleResource) {
        const RESOURCE_COUNT_LIMIT: usize = 64;
        const RESOURCE_BYTES_LIMIT: usize = 16 * 1024 * 1024;
        self.last_subsample_number = self.last_subsample_number.max(resource.index);
        if self.resources.contains_key(&resource.index) {
            return;
        }
        if self.resources.len() >= RESOURCE_COUNT_LIMIT
            || self.total_bytes.saturating_add(resource.bytes.len()) > RESOURCE_BYTES_LIMIT
        {
            self.overflowed = true;
            return;
        }
        self.total_bytes = self.total_bytes.saturating_add(resource.bytes.len());
        self.resources.insert(resource.index, resource);
    }

    pub(crate) fn is_complete(&self, last_subsample_number: u8) -> bool {
        !self.overflowed
            && (1..=last_subsample_number).all(|index| self.resources.contains_key(&index))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct BoundedResourceFormat {
    pub(crate) format_hint: Option<&'static str>,
    pub(crate) format_validation: &'static str,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

pub(crate) fn bounded_resource_format(bytes: &[u8]) -> BoundedResourceFormat {
    let signature_only = |format_hint| BoundedResourceFormat {
        format_hint: Some(format_hint),
        format_validation: "signature-only",
        width: None,
        height: None,
    };
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if bytes.len() >= 24 && &bytes[12..16] == b"IHDR" {
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            if (1..=16_384).contains(&width) && (1..=16_384).contains(&height) {
                return BoundedResourceFormat {
                    format_hint: Some("png"),
                    format_validation: "header-validated",
                    width: Some(width),
                    height: Some(height),
                };
            }
        }
        return signature_only("png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return signature_only("jpeg");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return signature_only("webp");
    }
    for (signature, format, minimum_length) in [
        (b"wOFF".as_slice(), "woff", 44_usize),
        (b"wOF2".as_slice(), "woff2", 48_usize),
    ] {
        if bytes.starts_with(signature) {
            let valid_length = bytes.len() >= minimum_length
                && u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize <= bytes.len();
            let table_count = bytes
                .get(12..14)
                .map(|value| u16::from_be_bytes(value.try_into().unwrap()));
            if valid_length && table_count.is_some_and(|count| (1..=4096).contains(&count)) {
                return BoundedResourceFormat {
                    format_hint: Some(format),
                    format_validation: "header-validated",
                    width: None,
                    height: None,
                };
            }
            return signature_only(format);
        }
    }
    if bytes.len() >= 12
        && (bytes.starts_with(&[0x00, 0x01, 0x00, 0x00]) || bytes.starts_with(b"OTTO"))
    {
        let format = if bytes.starts_with(b"OTTO") {
            "opentype"
        } else {
            "truetype"
        };
        let table_count = u16::from_be_bytes(bytes[4..6].try_into().unwrap());
        if (1..=4096).contains(&table_count) {
            return BoundedResourceFormat {
                format_hint: Some(format),
                format_validation: "header-validated",
                width: None,
                height: None,
            };
        }
        return signature_only(format);
    }
    BoundedResourceFormat {
        format_hint: None,
        format_validation: "not-identified",
        width: None,
        height: None,
    }
}

#[cfg(test)]
pub(crate) fn bounded_payload_format_hint(bytes: &[u8]) -> Option<&'static str> {
    bounded_resource_format(bytes).format_hint
}

pub(crate) fn bounded_png_preview_data_uri(bytes: &[u8]) -> Option<String> {
    const PREVIEW_BYTES_LIMIT: usize = 256 * 1024;
    let format = bounded_resource_format(bytes);
    if format.format_hint != Some("png")
        || format.format_validation != "header-validated"
        || bytes.len() > PREVIEW_BYTES_LIMIT
        || format.width.is_none_or(|width| width > 4096)
        || format.height.is_none_or(|height| height > 4096)
        || !png_has_bounded_image_chunks(bytes)
    {
        return None;
    }
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub(crate) fn png_has_bounded_image_chunks(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut position = 8_usize;
    let mut chunks = 0_usize;
    let mut has_idat = false;
    while position.saturating_add(12) <= bytes.len() && chunks < 16_384 {
        let length = usize::try_from(u32::from_be_bytes(
            bytes[position..position + 4].try_into().unwrap(),
        ))
        .ok();
        let Some(length) = length else { return false };
        let Some(end) = position
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
        else {
            return false;
        };
        if end > bytes.len() {
            return false;
        }
        let expected_crc = u32::from_be_bytes(bytes[end - 4..end].try_into().unwrap());
        if png_crc32(&bytes[position + 4..end - 4]) != expected_crc {
            return false;
        }
        match &bytes[position + 4..position + 8] {
            b"IDAT" => has_idat = true,
            b"IEND" => return length == 0 && has_idat,
            _ => {}
        }
        position = end;
        chunks += 1;
    }
    false
}

fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_state_is_bounded_and_tracks_completion() {
        let mut state = TlvSubtitleResourceState::default();
        state.add(TlvSubtitleResource {
            index: 1,
            data_type: 0,
            bytes: vec![1, 2, 3],
        });
        assert!(state.is_complete(1));
        assert_eq!(state.total_bytes, 3);
        state.add(TlvSubtitleResource {
            index: 1,
            data_type: 0,
            bytes: vec![9, 9],
        });
        assert_eq!(state.total_bytes, 3);
    }

    #[test]
    fn invalid_png_cannot_be_previewed() {
        assert!(bounded_png_preview_data_uri(b"\x89PNG\r\n\x1a\n").is_none());
    }

    #[test]
    fn resource_evidence_keeps_same_mpu_scope_and_lossless_bytes() {
        let evidence = ttml_resource_evidence(
            0x0459,
            7,
            &TlvSubtitleResource {
                index: 4,
                data_type: 1,
                bytes: vec![1, 2, 3],
            },
        );
        assert_eq!(evidence.scope_key, "packet:1113:mpu:7");
        assert_eq!(
            evidence.record_key,
            "stpp-resource:packet:1113:mpu:7:subsample:4"
        );
        assert_eq!(evidence.payload_base64, "AQID");
    }
}
