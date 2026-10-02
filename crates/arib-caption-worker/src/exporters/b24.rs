use super::*;

fn b24_interval_end(interval: &RegionInterval) -> Option<i64> {
    (interval.wait_duration_ms > 0 && interval.wait_duration_ms != i64::MAX)
        .then(|| interval.begin_ms.saturating_add(interval.wait_duration_ms))
}

pub(crate) fn flush_b24_flat_groups(
    writer: &mut BufWriter<File>,
    editable_writer: &mut Option<BufWriter<File>>,
    pending: &mut BTreeMap<(i64, i64), Vec<RegionInterval>>,
    active: &HashMap<RegionKey, RegionInterval>,
    flush_all: bool,
    options: &ConversionOptions,
) -> io::Result<()> {
    let ready = pending
        .keys()
        .copied()
        .filter(|timing| {
            flush_all
                || !active.values().any(|interval| {
                    interval.begin_ms == timing.0 && b24_interval_end(interval) == Some(timing.1)
                })
        })
        .collect::<Vec<_>>();
    for timing in ready {
        let intervals = pending.remove(&timing).unwrap_or_default();
        if options.preserve_position {
            if let Some(editable_writer) = editable_writer.as_mut() {
                write_ass_interval_group(editable_writer, &intervals, options)?;
            }
        } else {
            write_ass_interval_group(writer, &intervals, options)?;
        }
    }
    Ok(())
}

pub fn convert_b24_with_options_and_cancel<F, C>(
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
    let probe = probe_path(path)?;
    if probe.kind != InputKind::MpegTs {
        return Err(io::Error::other(
            "traditional B24 conversion requires an MPEG-TS recording",
        ));
    }
    let track = select_b24_track(discover_b24_tracks(path)?, options.track_id)?;
    let temporary = output.with_extension("ass.part");
    let drcs_directory = output.with_extension("drcs");
    let drcs_report_path = options
        .drcs_report
        .then(|| output.with_extension("drcs.json"));
    if drcs_report_path.as_ref().is_some_and(|path| path.exists()) && !options.overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "DRCS report already exists",
        ));
    }
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
            write_archive_header(&mut writer, path, "arib_std_b24")?;
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
            write_raw_header(&mut writer, path, "arib_std_b24")?;
            Some(writer)
        }
        None => None,
    };
    write_ass_header(&mut writer)?;
    let editable_temporary = (options.preserve_position && (options.srt || options.webvtt))
        .then(|| output.with_extension("editable.ass.part"));
    let mut editable_writer = match &editable_temporary {
        Some(path) => {
            let mut writer = BufWriter::new(File::create(path)?);
            write_ass_header(&mut writer)?;
            Some(writer)
        }
        None => None,
    };
    // Keep only the currently visible regions.  A full recording can be hundreds of
    // gigabytes long, while this state stays bounded by a single caption plane.
    let mut active_regions = HashMap::new();
    let mut final_scene_end = 0;
    let mut known_drcs = HashSet::new();
    let mut report_drcs = BTreeMap::new();
    let mut report_drcs_bytes = 0_usize;
    let mut have_drcs = false;
    let mut pending_flat = BTreeMap::<(i64, i64), Vec<RegionInterval>>::new();
    let mut semantic_state = crate::caption_features::CaptionSequenceState::default();
    let mut last_scene_pid = track.caption_pid;
    let summary = match scan_b24(
        path,
        &track,
        |source_pid, scene| {
            last_scene_pid = source_pid;
            if options.drcs_report {
                for glyph in &scene.drcs_glyphs {
                    let unresolved = glyph.alternative_text.is_empty()
                        && (options.drcs_mode != crate::DrcsMode::UseUserMapping
                            || options
                                .drcs_replacements
                                .get(&glyph.drcs_code)
                                .is_none_or(String::is_empty));
                    let key = drcs_asset_key(glyph);
                    if !unresolved
                        || report_drcs.contains_key(&key)
                        || report_drcs.len() >= 64
                        || report_drcs_bytes.saturating_add(glyph.pixels.len()) > 32 * 1024 * 1024
                    {
                        continue;
                    }
                    have_drcs |= write_drcs_asset(&drcs_directory, glyph, &mut known_drcs)?;
                    report_drcs_bytes = report_drcs_bytes.saturating_add(glyph.pixels.len());
                    report_drcs.insert(key, glyph.clone());
                }
            }
            assess_b24_scene(&options, &scene)?;
            if let Some(archive_writer) = &mut archive_writer {
                write_archive_record(archive_writer, "scene", &scene)?;
            }
            final_scene_end = caption_end(
                scene.pts_ms,
                scene.wait_duration_ms,
                scene.pts_ms.saturating_add(5_000),
            );
            for mut interval in
                apply_scene_intervals(&mut active_regions, &scene, &mut semantic_state)
            {
                interval.source_pid = Some(source_pid);
                if options.preserve_position {
                    write_ass_interval(&mut writer, &interval, &options)?;
                }
                if !options.preserve_position || editable_writer.is_some() {
                    pending_flat
                        .entry((interval.begin_ms, interval.end_ms))
                        .or_default()
                        .push(interval.clone());
                }
                if let Some(ttml_writer) = &mut ttml_writer {
                    write_ttml_interval(ttml_writer, &interval, &options)?;
                }
                if let Some(archive_writer) = &mut archive_writer {
                    write_caption_archive_record(archive_writer, CaptionCueRef::B24(&interval))?;
                }
            }
            flush_b24_flat_groups(
                &mut writer,
                &mut editable_writer,
                &mut pending_flat,
                &active_regions,
                false,
                &options,
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
    ) {
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
            if let Some(path) = &editable_temporary {
                let _ = fs::remove_file(path);
            }
            if options.drcs_report
                && !report_drcs.is_empty()
                && write_drcs_report(output, path, &drcs_directory, &report_drcs, true)?.is_some()
            {
                return Err(crate::export_assessment::with_drcs_report_created(error));
            }
            return Err(error);
        }
    };
    for mut interval in finish_scene_intervals(&mut active_regions, final_scene_end) {
        interval.source_pid = Some(last_scene_pid);
        if options.preserve_position {
            write_ass_interval(&mut writer, &interval, &options)?;
        }
        if !options.preserve_position || editable_writer.is_some() {
            pending_flat
                .entry((interval.begin_ms, interval.end_ms))
                .or_default()
                .push(interval.clone());
        }
        if let Some(ttml_writer) = &mut ttml_writer {
            write_ttml_interval(ttml_writer, &interval, &options)?;
        }
        if let Some(archive_writer) = &mut archive_writer {
            write_caption_archive_record(archive_writer, CaptionCueRef::B24(&interval))?;
        }
    }
    flush_b24_flat_groups(
        &mut writer,
        &mut editable_writer,
        &mut pending_flat,
        &active_regions,
        true,
        &options,
    )?;
    let drcs_report = if options.drcs_report {
        write_drcs_report(
            output,
            path,
            &drcs_directory,
            &report_drcs,
            options.overwrite,
        )?
    } else {
        None
    };
    if let Some(editable_writer) = editable_writer.as_mut() {
        editable_writer.flush()?;
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
    let (ass, font_directory, srt, webvtt) =
        finalize_ass_outputs(output, editable_temporary.as_deref(), &options)?;
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
        drcs_directory: have_drcs.then_some(drcs_directory),
        drcs_report,
        ttml,
        archive,
        raw,
        srt,
        webvtt,
        summary,
    })
}

pub(crate) fn select_b24_track(
    tracks: Vec<B24Track>,
    track_id: Option<u16>,
) -> io::Result<B24Track> {
    let track = match track_id {
        Some(track_id) => tracks
            .into_iter()
            .find(|track| track.caption_pids.contains(&track_id)),
        None => tracks.into_iter().next(),
    };
    track.ok_or_else(|| {
        io::Error::other(match track_id {
            Some(track_id) => {
                format!("requested track_id 0x{track_id:04X} was not discovered in this recording")
            }
            None => "no traditional B24 caption track found".into(),
        })
    })
}
