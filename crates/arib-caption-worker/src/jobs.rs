use crate::*;
use std::cell::RefCell;

fn queue_ass_ttml_caption(
    writer: &mut BufWriter<File>,
    pending: &mut Vec<TtmlCaption>,
    archive_writer: &mut Option<BufWriter<File>>,
    ttml_writer: &mut Option<BufWriter<File>>,
    options: &ConversionOptions,
    caption: TtmlCaption,
) -> io::Result<()> {
    let same_group = pending
        .first()
        .is_none_or(|first| first.start_ms == caption.start_ms && first.end_ms == caption.end_ms);
    if !same_group {
        flush_ass_ttml_group(writer, pending, archive_writer, ttml_writer, options)?;
    }
    pending.push(caption);
    Ok(())
}

fn flush_ass_ttml_group(
    writer: &mut BufWriter<File>,
    pending: &mut Vec<TtmlCaption>,
    archive_writer: &mut Option<BufWriter<File>>,
    ttml_writer: &mut Option<BufWriter<File>>,
    options: &ConversionOptions,
) -> io::Result<()> {
    associate_standalone_ttml_ruby(pending);
    for caption in pending.iter() {
        if let Some(archive_writer) = archive_writer.as_mut() {
            write_caption_archive_record(archive_writer, CaptionCueRef::AribTtml(caption))?;
        }
        if let Some(ttml_writer) = ttml_writer.as_mut() {
            write_ttml_caption(ttml_writer, caption, options)?;
        }
    }
    write_ass_ttml_group(writer, pending, options)?;
    pending.clear();
    Ok(())
}

#[derive(Clone, Copy)]
enum TtmlPesPacketisation {
    MpegTs188,
    M2ts192,
}

pub(crate) fn convert_mpeg_ts_ttml_with_options_and_cancel<F, C>(
    path: &Path,
    output: &Path,
    options: ConversionOptions,
    progress: F,
    cancelled: C,
) -> io::Result<ConversionReport>
where
    F: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
{
    convert_ttml_pes_with_options_and_cancel(
        path,
        output,
        options,
        TtmlPesPacketisation::MpegTs188,
        progress,
        cancelled,
    )
}

pub(crate) fn convert_m2ts_ttml_with_options_and_cancel<F, C>(
    path: &Path,
    output: &Path,
    options: ConversionOptions,
    progress: F,
    cancelled: C,
) -> io::Result<ConversionReport>
where
    F: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
{
    convert_ttml_pes_with_options_and_cancel(
        path,
        output,
        options,
        TtmlPesPacketisation::M2ts192,
        progress,
        cancelled,
    )
}

fn convert_ttml_pes_with_options_and_cancel<F, C>(
    path: &Path,
    output: &Path,
    options: ConversionOptions,
    packetisation: TtmlPesPacketisation,
    mut progress: F,
    mut cancelled: C,
) -> io::Result<ConversionReport>
where
    F: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
{
    if output.exists() && !options.overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "output file already exists",
        ));
    }
    let mut tracks = match packetisation {
        TtmlPesPacketisation::MpegTs188 => discover_mpeg_ts_data_tracks(path)?,
        TtmlPesPacketisation::M2ts192 => discover_m2ts_data_tracks(path)?,
    }
    .ok_or_else(|| io::Error::other("no private data PID found for an ARIB-TTML scan"))?;
    if let Some(track_id) = options.track_id {
        if !tracks.pids.contains(&track_id) {
            return Err(io::Error::other(format!(
                "requested track_id 0x{track_id:04X} was not discovered in this recording"
            )));
        }
        tracks.pids.retain(|pid| *pid == track_id);
    } else {
        tracks.retain_default_caption_tracks();
    }
    let temporary = output.with_extension("ass.part");
    let mut writer = BufWriter::new(File::create(&temporary)?);
    let ttml = options.ttml.then(|| output.with_extension("ttml"));
    let ttml_temporary = ttml.as_ref().map(|path| path.with_extension("ttml.part"));
    let mut ttml_writer = match &ttml_temporary {
        Some(path) => {
            let mut writer = BufWriter::new(File::create(path)?);
            write_ttml_header(&mut writer)?;
            Some(writer)
        }
        None => None,
    };
    let archive = options
        .archive
        .then(|| output.with_extension("caption.jsonl"));
    let archive_temporary = archive
        .as_ref()
        .map(|path| path.with_extension("jsonl.part"));
    let mut archive_writer = match &archive_temporary {
        Some(temporary_path) => {
            let mut writer = BufWriter::new(File::create(temporary_path)?);
            write_archive_header(&mut writer, path, "arib_ttml")?;
            Some(writer)
        }
        None => None,
    };
    let raw = options
        .raw
        .then(|| output.with_extension("caption.pes.jsonl"));
    let raw_temporary = raw.as_ref().map(|path| path.with_extension("jsonl.part"));
    let mut raw_writer = match &raw_temporary {
        Some(temporary_path) => {
            let mut writer = BufWriter::new(File::create(temporary_path)?);
            write_raw_header(&mut writer, path, "arib_ttml_private_pes")?;
            Some(writer)
        }
        None => None,
    };
    write_ass_header(&mut writer)?;
    let mut pending_ass = Vec::new();
    let scan = match packetisation {
        TtmlPesPacketisation::MpegTs188 => scan_mpeg_ts_ttml(
            path,
            &tracks,
            |caption| {
                queue_ass_ttml_caption(
                    &mut writer,
                    &mut pending_ass,
                    &mut archive_writer,
                    &mut ttml_writer,
                    &options,
                    caption,
                )?;
                Ok(())
            },
            |summary| progress(summary),
            &mut cancelled,
            |pid, packet_offset, pes| {
                if let Some(raw_writer) = &mut raw_writer {
                    write_raw_pes_record(raw_writer, pid, packet_offset, pes)?;
                }
                Ok(())
            },
        ),
        TtmlPesPacketisation::M2ts192 => scan_m2ts_ttml(
            path,
            &tracks,
            |caption| {
                queue_ass_ttml_caption(
                    &mut writer,
                    &mut pending_ass,
                    &mut archive_writer,
                    &mut ttml_writer,
                    &options,
                    caption,
                )?;
                Ok(())
            },
            |summary| progress(summary),
            cancelled,
            |pid, packet_offset, pes| {
                if let Some(raw_writer) = &mut raw_writer {
                    write_raw_pes_record(raw_writer, pid, packet_offset, pes)?;
                }
                Ok(())
            },
        ),
    };
    let summary = match scan {
        Ok(mut summary) => {
            summary.features.complete = true;
            summary
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            if let Some(path) = &archive_temporary {
                let _ = fs::remove_file(path);
            }
            if let Some(path) = &ttml_temporary {
                let _ = fs::remove_file(path);
            }
            if let Some(path) = &raw_temporary {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
    };
    flush_ass_ttml_group(
        &mut writer,
        &mut pending_ass,
        &mut archive_writer,
        &mut ttml_writer,
        &options,
    )?;
    writer.flush()?;
    publish_file(&temporary, output, options.overwrite)?;
    if let (Some(mut ttml_writer), Some(ttml), Some(ttml_temporary)) =
        (ttml_writer, ttml.as_ref(), ttml_temporary.as_ref())
    {
        write_ttml_footer(&mut ttml_writer)?;
        ttml_writer.flush()?;
        publish_file(ttml_temporary, ttml, options.overwrite)?;
    }
    if let (Some(mut archive_writer), Some(archive), Some(archive_temporary)) =
        (archive_writer, archive.as_ref(), archive_temporary.as_ref())
    {
        write_archive_record(&mut archive_writer, "summary", &summary)?;
        archive_writer.flush()?;
        publish_file(archive_temporary, archive, options.overwrite)?;
    }
    if let (Some(mut raw_writer), Some(raw), Some(raw_temporary)) =
        (raw_writer, raw.as_ref(), raw_temporary.as_ref())
    {
        raw_writer.flush()?;
        publish_file(raw_temporary, raw, options.overwrite)?;
    }
    let (ass, font_directory, srt, webvtt) = finalize_ass_outputs(output, &options)?;
    let primary = ass
        .as_ref()
        .or(ttml.as_ref())
        .or(srt.as_ref())
        .or(webvtt.as_ref())
        .or(archive.as_ref())
        .or(raw.as_ref())
        .cloned()
        .unwrap_or_else(|| output.to_path_buf());
    Ok(ConversionReport {
        output: primary,
        ass,
        font_directory,
        drcs_directory: None,
        drcs_report: None,
        ttml,
        archive,
        raw,
        srt,
        webvtt,
        summary,
    })
}

pub(crate) fn convert_tlv_ttml_with_options_and_cancel<F, C>(
    path: &Path,
    output: &Path,
    options: ConversionOptions,
    mut progress: F,
    cancelled: C,
) -> io::Result<ConversionReport>
where
    F: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
{
    if output.exists() && !options.overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "output file already exists",
        ));
    }
    let temporary = output.with_extension("ass.part");
    let mut writer = BufWriter::new(File::create(&temporary)?);
    let ttml = options.ttml.then(|| output.with_extension("ttml"));
    let ttml_temporary = ttml.as_ref().map(|path| path.with_extension("ttml.part"));
    let mut ttml_writer = match &ttml_temporary {
        Some(path) => {
            let mut writer = BufWriter::new(File::create(path)?);
            write_ttml_header(&mut writer)?;
            Some(writer)
        }
        None => None,
    };
    let archive = options
        .archive
        .then(|| output.with_extension("caption.jsonl"));
    let archive_temporary = archive
        .as_ref()
        .map(|path| path.with_extension("jsonl.part"));
    let archive_writer = RefCell::new(match &archive_temporary {
        Some(temporary_path) => {
            let mut writer = BufWriter::new(File::create(temporary_path)?);
            write_archive_header(&mut writer, path, "isdb_s3_mmt_stpp_ttml")?;
            Some(writer)
        }
        None => None,
    });
    let raw = options
        .raw
        .then(|| output.with_extension("caption.mmtp.jsonl"));
    let raw_temporary = raw.as_ref().map(|path| path.with_extension("jsonl.part"));
    let mut raw_writer = match &raw_temporary {
        Some(path) => {
            let mut writer = BufWriter::new(File::create(path)?);
            write_tlv_raw_header(&mut writer, path)?;
            Some(writer)
        }
        None => None,
    };
    let mut archived_resource_references = BTreeSet::new();
    let mut archived_resource_evidence = BTreeSet::new();
    let mut archived_assets = Vec::new();
    write_ass_header(&mut writer)?;
    let mut pending_ass = Vec::new();
    let mut feature_summary = CaptionFeatureSummary::default();
    let summary = match scan_tlv_ttml(
        path,
        |caption| {
            feature_summary.observe_ttml(&caption);
            let mut archive = archive_writer.borrow_mut();
            if let Some(archive_writer) = &mut *archive {
                for resource in ttml_resource_references(&caption) {
                    // A reference can be repeated by every caption in a long
                    // programme. Keep the archive streaming and bounded by
                    // writing each URI/usage pair once.
                    let key = format!(
                        "{}:{}:{}",
                        resource.usage,
                        resource.uri,
                        resource
                            .association
                            .scope_key
                            .as_deref()
                            .unwrap_or("scope:unknown")
                    );
                    if archived_resource_references.insert(key) {
                        write_archive_record(archive_writer, "resource_reference", &resource)?;
                    }
                }
            }
            queue_ass_ttml_caption(
                &mut writer,
                &mut pending_ass,
                &mut archive,
                &mut ttml_writer,
                &options,
                caption,
            )?;
            Ok(())
        },
        |summary| progress(summary),
        cancelled,
        |packet_offset, payload| {
            if let Some(archive_writer) = &mut *archive_writer.borrow_mut()
                && let Some(mpu_sequence_number) = payload.mpu_sequence_number
            {
                for resource in &payload.resources {
                    let evidence =
                        ttml_resource_evidence(payload.packet_id, mpu_sequence_number, resource);
                    if archived_resource_evidence.insert(evidence.record_key.clone()) {
                        write_archive_record(archive_writer, "resource_evidence", &evidence)?;
                    }
                }
            }
            if let Some(raw_writer) = &mut raw_writer {
                write_tlv_raw_payload(raw_writer, packet_offset, payload)?;
            }
            Ok(())
        },
        |asset| {
            archived_assets.push(asset);
            Ok(())
        },
    ) {
        Ok(mut summary) => {
            summary.features = feature_summary;
            summary.features.complete = true;
            summary
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            if let Some(path) = &archive_temporary {
                let _ = fs::remove_file(path);
            }
            if let Some(path) = &ttml_temporary {
                let _ = fs::remove_file(path);
            }
            if let Some(path) = &raw_temporary {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
    };
    {
        let mut archive = archive_writer.borrow_mut();
        flush_ass_ttml_group(
            &mut writer,
            &mut pending_ass,
            &mut archive,
            &mut ttml_writer,
            &options,
        )?;
    }
    if let Some(archive_writer) = &mut *archive_writer.borrow_mut() {
        for asset in archived_assets {
            write_archive_record(archive_writer, "asset_evidence", &asset)?;
        }
    }
    writer.flush()?;
    publish_file(&temporary, output, options.overwrite)?;
    if let (Some(mut ttml_writer), Some(ttml), Some(ttml_temporary)) =
        (ttml_writer, ttml.as_ref(), ttml_temporary.as_ref())
    {
        write_ttml_footer(&mut ttml_writer)?;
        ttml_writer.flush()?;
        publish_file(ttml_temporary, ttml, options.overwrite)?;
    }
    if let (Some(mut archive_writer), Some(archive), Some(archive_temporary)) = (
        archive_writer.into_inner(),
        archive.as_ref(),
        archive_temporary.as_ref(),
    ) {
        write_archive_record(&mut archive_writer, "summary", &summary)?;
        archive_writer.flush()?;
        publish_file(archive_temporary, archive, options.overwrite)?;
    }
    if let (Some(mut raw_writer), Some(raw), Some(raw_temporary)) =
        (raw_writer, raw.as_ref(), raw_temporary.as_ref())
    {
        raw_writer.flush()?;
        publish_file(raw_temporary, raw, options.overwrite)?;
    }
    let (ass, font_directory, srt, webvtt) = finalize_ass_outputs(output, &options)?;
    let primary = ass
        .as_ref()
        .or(ttml.as_ref())
        .or(srt.as_ref())
        .or(webvtt.as_ref())
        .or(archive.as_ref())
        .or(raw.as_ref())
        .cloned()
        .unwrap_or_else(|| output.to_path_buf());
    Ok(ConversionReport {
        output: primary,
        ass,
        font_directory,
        drcs_directory: None,
        drcs_report: None,
        ttml,
        archive,
        raw,
        srt,
        webvtt,
        summary,
    })
}

pub fn convert_with_options_and_cancel<F, C>(
    path: &Path,
    output: &Path,
    options: ConversionOptions,
    progress: F,
    cancelled: C,
) -> io::Result<ConversionReport>
where
    F: FnMut(&B24DecodeSummary),
    C: FnMut() -> bool,
{
    match probe_path(path)?.kind {
        InputKind::MpegTs => {
            let b24_tracks = discover_b24_tracks(path)?;
            let use_b24 = match options.track_id {
                Some(track_id) => b24_tracks.iter().any(|track| track.caption_pid == track_id),
                None => !b24_tracks.is_empty(),
            };
            if use_b24 {
                convert_b24_with_options_and_cancel(path, output, options, progress, cancelled)
            } else {
                convert_mpeg_ts_ttml_with_options_and_cancel(
                    path, output, options, progress, cancelled,
                )
            }
        }
        InputKind::M2ts => {
            convert_m2ts_ttml_with_options_and_cancel(path, output, options, progress, cancelled)
        }
        InputKind::Tlv => {
            convert_tlv_ttml_with_options_and_cancel(path, output, options, progress, cancelled)
        }
        InputKind::Unknown => Err(io::Error::other(
            "unsupported or unrecognised recording container",
        )),
    }
}
