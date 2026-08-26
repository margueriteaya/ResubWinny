use super::*;

pub(crate) fn write_webvtt_from_ass(ass: &Path, overwrite: bool) -> io::Result<Option<PathBuf>> {
    let vtt = ass.with_extension("vtt");
    if vtt.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "WebVTT output already exists",
        ));
    }
    let temporary = vtt.with_extension("vtt.part");
    let mut writer = BufWriter::new(File::create(&temporary)?);
    writeln!(writer, "WEBVTT\n")?;
    for line in std::io::BufRead::lines(BufReader::new(File::open(ass)?)) {
        let line = line?;
        let Some(body) = line.strip_prefix("Dialogue: ") else {
            continue;
        };
        let parts: Vec<_> = body.splitn(10, ',').collect();
        if parts.len() != 10 {
            continue;
        }
        let text = ass_to_webvtt_text(parts[9]);
        writeln!(
            writer,
            "{} --> {}\n{}\n",
            ass_time_to_vtt(parts[1]),
            ass_time_to_vtt(parts[2]),
            text
        )?;
    }
    writer.flush()?;
    publish_file(&temporary, &vtt, overwrite)?;
    Ok(Some(vtt))
}

pub(crate) fn write_srt_from_ass(ass: &Path, overwrite: bool) -> io::Result<Option<PathBuf>> {
    let srt = ass.with_extension("srt");
    if srt.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "SRT output already exists",
        ));
    }
    let temporary = srt.with_extension("srt.part");
    let mut writer = BufWriter::new(File::create(&temporary)?);
    let mut cue = 0_u64;
    for line in std::io::BufRead::lines(BufReader::new(File::open(ass)?)) {
        let line = line?;
        let Some(body) = line.strip_prefix("Dialogue: ") else {
            continue;
        };
        let parts: Vec<_> = body.splitn(10, ',').collect();
        if parts.len() != 10 {
            continue;
        }
        let text = ass_to_webvtt_text(parts[9]);
        if text.trim().is_empty() {
            continue;
        }
        cue = cue.saturating_add(1);
        writeln!(
            writer,
            "{cue}\n{} --> {}\n{text}\n",
            ass_time_to_srt(parts[1]),
            ass_time_to_srt(parts[2]),
        )?;
    }
    writer.flush()?;
    publish_file(&temporary, &srt, overwrite)?;
    Ok(Some(srt))
}

pub(crate) fn ass_to_webvtt_text(text: &str) -> String {
    let mut output = String::new();
    let mut drawing_mode = false;
    let mut saw_drawing = false;
    let mut remaining = text;
    while let Some(start) = remaining.find('{') {
        if !drawing_mode {
            output.push_str(&remaining[..start]);
        }
        let Some(end) = remaining[start + 1..].find('}') else {
            if !drawing_mode {
                output.push_str(&remaining[start..]);
            }
            break;
        };
        let tag = &remaining[start + 1..start + 1 + end];
        if let Some(enabled) = ass_drawing_mode(tag) {
            drawing_mode = enabled;
            saw_drawing |= enabled;
        }
        remaining = &remaining[start + 2 + end..];
    }
    if !drawing_mode {
        output.push_str(remaining);
    }
    let output = output.replace("\\N", "\n");
    if output.trim().is_empty() && saw_drawing {
        "[DRCS glyph]".to_owned()
    } else {
        output
    }
}

pub(crate) fn ass_drawing_mode(tag: &str) -> Option<bool> {
    tag.match_indices("\\p").find_map(|(index, _)| {
        let mode = tag[index + 2..].chars().next()?;
        mode.is_ascii_digit().then_some(mode != '0')
    })
}

pub(crate) fn ass_time_to_vtt(value: &str) -> String {
    let (clock, hundredths) = value.rsplit_once('.').unwrap_or((value, "0"));
    let mut parts = clock.split(':');
    let hours = parts.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
    let minutes = parts.next().unwrap_or("0");
    let seconds = parts.next().unwrap_or("0");
    format!("{hours:02}:{minutes:0>2}:{seconds:0>2}.{hundredths:0<2}0")
}

pub(crate) fn ass_time_to_srt(value: &str) -> String {
    ass_time_to_vtt(value).replace('.', ",")
}

type AssOutputPaths = (
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
);

pub(crate) fn finalize_ass_outputs(
    output: &Path,
    options: &ConversionOptions,
) -> io::Result<AssOutputPaths> {
    let srt = options
        .srt
        .then(|| write_srt_from_ass(output, options.overwrite))
        .transpose()?
        .flatten();
    let webvtt = options
        .webvtt
        .then(|| write_webvtt_from_ass(output, options.overwrite))
        .transpose()?
        .flatten();
    if options.keep_ass {
        let font_directory = options
            .preserve_gaiji
            .then(|| write_ass_font_directory(output, options.overwrite))
            .transpose()?;
        Ok((Some(output.to_path_buf()), font_directory, srt, webvtt))
    } else {
        fs::remove_file(output)?;
        Ok((None, None, srt, webvtt))
    }
}
