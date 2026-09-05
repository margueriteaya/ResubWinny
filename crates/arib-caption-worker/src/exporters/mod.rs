use crate::*;

mod ass;
mod ass_rich;
mod b24;
mod evidence;
mod ruby_layout;
mod text;
mod ttml;

pub(crate) use ass::*;
#[cfg(test)]
pub(crate) use ass_rich::bundled_ass_font;
pub(crate) use b24::*;
pub(crate) use evidence::*;
pub(crate) use text::*;
pub(crate) use ttml::*;

pub(crate) fn keep_text(value: &str, options: &ConversionOptions) -> bool {
    !export_text(value, options).is_empty()
}

pub(crate) fn export_text(value: &str, options: &ConversionOptions) -> String {
    crate::caption_features::filtered_text(
        value,
        options.preserve_gaiji,
        options.preserve_accessibility,
    )
}

pub(crate) fn export_ttml_text(
    value: &str,
    style: &TtmlCaptionStyle,
    options: &ConversionOptions,
) -> String {
    let resource_backed = style
        .font_resource
        .as_deref()
        .and_then(subt_resource_index)
        .is_some();
    let text = value
        .chars()
        .filter_map(|character| {
            if !resource_backed || ttml_drcs_kind(character).is_none() {
                return Some(character.to_string());
            }
            if !options.preserve_drcs {
                return None;
            }
            Some(character.to_string())
        })
        .collect::<String>();
    export_text(&text, options)
}

pub(crate) fn publish_file(temporary: &Path, output: &Path, overwrite: bool) -> io::Result<()> {
    if !overwrite || !output.exists() {
        return fs::rename(temporary, output);
    }
    let backup = output.with_extension(format!(
        "{}.backup",
        output
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("output")
    ));
    if backup.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "existing backup file blocks overwrite",
        ));
    }
    fs::rename(output, &backup)?;
    if let Err(error) = fs::rename(temporary, output) {
        let _ = fs::rename(&backup, output);
        return Err(error);
    }
    fs::remove_file(backup)
}
