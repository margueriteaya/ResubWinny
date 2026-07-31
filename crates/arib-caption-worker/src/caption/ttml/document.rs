use crate::*;

pub(crate) fn ttml_documents(pes: &[u8]) -> Vec<DecodedTtmlDocument> {
    let mut documents = Vec::new();
    let mut remaining = pes;
    while let Some((start, hint)) = ttml_document_start_bytes(remaining) {
        let candidate = &remaining[start..];
        let Some(encoding) = xml_encoding_for_candidate(candidate, hint) else {
            // A declared but unsupported/invalid document must not be retried
            // from its later `<tt>` root as if it were declaration-free UTF-8.
            // Skip that candidate and continue looking for the next document.
            let skipped = find_bytes(candidate, hint.closing_tag())
                .map(|end| end + hint.closing_tag().len())
                .unwrap_or(1);
            remaining = &candidate[skipped..];
            continue;
        };
        let closing = encoding.closing_tag();
        let Some(end) = find_bytes(candidate, closing) else {
            break;
        };
        let raw_document = &candidate[..end + closing.len()];
        if let Some(xml) = decode_xml_bytes(raw_document, encoding)
            && is_complete_ttml_document(&xml)
        {
            documents.push(DecodedTtmlDocument { xml, encoding });
        }
        remaining = &candidate[end + closing.len()..];
    }
    documents
}

pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    (!needle.is_empty())
        .then(|| {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        })
        .flatten()
}

pub(crate) fn ttml_document_start_bytes(value: &[u8]) -> Option<(usize, XmlTextEncoding)> {
    [
        (find_ascii_ttml_start(value), XmlTextEncoding::Utf8),
        (find_utf16_ttml_start(value, true), XmlTextEncoding::Utf16Le),
        (
            find_utf16_ttml_start(value, false),
            XmlTextEncoding::Utf16Be,
        ),
    ]
    .into_iter()
    .filter_map(|(offset, encoding)| offset.map(|offset| (offset, encoding)))
    .min_by_key(|(offset, _)| *offset)
}

pub(crate) fn find_ascii_ttml_start(value: &[u8]) -> Option<usize> {
    let declaration = find_bytes(value, b"<?xml");
    let root = value.windows(3).enumerate().find_map(|(index, window)| {
        (window == b"<tt")
            .then(|| value.get(index + 3).copied())
            .flatten()
            .filter(|byte| byte.is_ascii_whitespace() || matches!(*byte, b'>' | b'/'))
            .map(|_| index)
    });
    match (declaration, root) {
        (Some(declaration), Some(root)) => Some(declaration.min(root)),
        (Some(declaration), None) => Some(declaration),
        (None, Some(root)) => Some(root),
        (None, None) => None,
    }
}

pub(crate) fn find_utf16_ttml_start(value: &[u8], little_endian: bool) -> Option<usize> {
    let (declaration, root) = if little_endian {
        (
            find_bytes(value, b"<\0?\0x\0m\0l\0"),
            find_bytes(value, b"<\0t\0t\0"),
        )
    } else {
        (
            find_bytes(value, b"\0<\0?\0x\0m\0l"),
            find_bytes(value, b"\0<\0t\0t"),
        )
    };
    match (declaration, root) {
        (Some(declaration), Some(root)) => Some(declaration.min(root)),
        (Some(declaration), None) => Some(declaration),
        (None, Some(root)) => Some(root),
        (None, None) => None,
    }
}

pub(crate) fn xml_encoding_for_candidate(
    candidate: &[u8],
    detected: XmlTextEncoding,
) -> Option<XmlTextEncoding> {
    // A Shift_JIS/EUC-JP declaration is ASCII, while its body is not UTF-8.
    // Read only that ASCII declaration first; decoding the whole candidate as
    // UTF-8 here would recreate the old "one invalid byte drops all captions"
    // failure mode before we have learned its declared encoding.
    let declaration = if detected == XmlTextEncoding::Utf8 && candidate.starts_with(b"<?xml") {
        let end = find_bytes(candidate, b"?>")?;
        std::str::from_utf8(&candidate[..end + 2]).ok()?.to_owned()
    } else if detected == XmlTextEncoding::Utf8 {
        String::new()
    } else {
        decode_xml_bytes(candidate, detected)?
    };
    let declared = xml_declared_encoding(&declaration);
    match declared.as_deref() {
        None => Some(detected),
        Some("utf-8") | Some("utf8") => Some(XmlTextEncoding::Utf8),
        Some("utf-16") | Some("utf-16le") | Some("utf-16-le") => {
            matches!(detected, XmlTextEncoding::Utf16Le).then_some(XmlTextEncoding::Utf16Le)
        }
        Some("utf-16be") | Some("utf-16-be") => {
            matches!(detected, XmlTextEncoding::Utf16Be).then_some(XmlTextEncoding::Utf16Be)
        }
        Some("shift_jis") | Some("shift-jis") | Some("sjis") => Some(XmlTextEncoding::ShiftJis),
        Some("euc-jp") | Some("euc_jp") => Some(XmlTextEncoding::EucJp),
        Some("iso-2022-jp") | Some("iso_2022_jp") => Some(XmlTextEncoding::Iso2022Jp),
        Some(_) => None,
    }
}

pub(crate) fn decode_xml_bytes(raw: &[u8], encoding: XmlTextEncoding) -> Option<String> {
    let raw = match encoding {
        XmlTextEncoding::Utf8 => raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(raw),
        XmlTextEncoding::Utf16Le => raw.strip_prefix(&[0xff, 0xfe]).unwrap_or(raw),
        XmlTextEncoding::Utf16Be => raw.strip_prefix(&[0xfe, 0xff]).unwrap_or(raw),
        _ => raw,
    };
    let (decoded, had_errors) = match encoding {
        XmlTextEncoding::Utf8 => (String::from_utf8(raw.to_vec()).ok()?, false),
        XmlTextEncoding::Utf16Le => {
            let (text, had_errors) = encoding_rs::UTF_16LE.decode_without_bom_handling(raw);
            (text.into_owned(), had_errors)
        }
        XmlTextEncoding::Utf16Be => {
            let (text, had_errors) = encoding_rs::UTF_16BE.decode_without_bom_handling(raw);
            (text.into_owned(), had_errors)
        }
        XmlTextEncoding::ShiftJis => {
            let (text, had_errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(raw);
            (text.into_owned(), had_errors)
        }
        XmlTextEncoding::EucJp => {
            let (text, had_errors) = encoding_rs::EUC_JP.decode_without_bom_handling(raw);
            (text.into_owned(), had_errors)
        }
        XmlTextEncoding::Iso2022Jp => {
            let (text, had_errors) = encoding_rs::ISO_2022_JP.decode_without_bom_handling(raw);
            (text.into_owned(), had_errors)
        }
    };
    (!had_errors).then_some(decoded)
}

pub(crate) fn xml_declared_encoding(xml: &str) -> Option<String> {
    let declaration_end = xml.find("?>")?;
    let declaration = xml[..declaration_end].to_ascii_lowercase();
    let marker = declaration.find("encoding")?;
    let rest = declaration[marker + "encoding".len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let quote = rest
        .chars()
        .next()
        .filter(|quote| matches!(*quote, '\"' | '\''))?;
    let value = &rest[quote.len_utf8()..];
    value.find(quote).map(|end| value[..end].to_owned())
}

pub(crate) fn is_complete_ttml_document(xml: &str) -> bool {
    let Some(root) = xml.find("<tt") else {
        return false;
    };
    let after = xml.as_bytes().get(root + 3).copied();
    matches!(after, Some(byte) if byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
        && xml[root..].contains("</tt>")
}
