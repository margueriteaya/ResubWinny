use crate::*;
use std::io::BufRead;

#[derive(Debug, Serialize)]
pub(crate) struct ArchivePreview {
    pub(crate) source: PathBuf,
    pub(crate) time_ms: i64,
    pub(crate) intervals: Vec<serde_json::Value>,
}

pub(crate) fn render_archive_at(path: &Path, time_ms: i64) -> io::Result<ArchivePreview> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut active: Vec<(i64, serde_json::Value)> = Vec::new();
    let mut active_scene: Option<serde_json::Value> = None;
    let mut line = String::new();
    let mut first_line = true;
    while reader.read_line(&mut line)? > 0 {
        let envelope = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(value) => value,
            Err(_) => {
                line.clear();
                continue;
            }
        };
        if first_line {
            first_line = false;
            validate_archive_header(&envelope)?;
        }
        let kind = envelope
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .or_else(|| envelope.get("type").and_then(serde_json::Value::as_str))
            .map(str::to_owned);
        if !matches!(
            kind.as_deref(),
            Some("region_interval") | Some("caption") | Some("scene")
        ) {
            line.clear();
            continue;
        }
        let value = envelope.get("value").cloned().unwrap_or(envelope);
        let Some((begin, end)) =
            interval_bounds(&value).or_else(|| scene_bounds(&value, kind.as_deref()))
        else {
            line.clear();
            continue;
        };
        if begin > time_ms {
            break;
        }
        if kind.as_deref() == Some("scene") {
            // B24 scene records are whole-plane replacement snapshots. They
            // must never be composed with an earlier scene at the same time.
            active_scene = (time_ms < end).then_some(value);
        } else {
            active.retain(|(active_end, _)| time_ms < *active_end);
            if time_ms < end && active.len() < 128 {
                active.push((end, value));
            }
        }
        line.clear();
    }
    Ok(ArchivePreview {
        source: path.to_path_buf(),
        time_ms,
        intervals: active_scene
            .map(|scene| vec![scene])
            .unwrap_or_else(|| active.into_iter().map(|(_, value)| value).collect()),
    })
}

fn validate_archive_header(envelope: &serde_json::Value) -> io::Result<()> {
    let is_header = envelope.get("type").and_then(serde_json::Value::as_str)
        == Some("arib_caption_studio_archive");
    if !is_header {
        return Ok(());
    }
    let explicit = envelope
        .get("schemaVersion")
        .or_else(|| envelope.get("schema_version"));
    if let (Some(explicit), Some(legacy)) = (explicit, envelope.get("version"))
        && explicit != legacy
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Caption archive schema version fields disagree",
        ));
    }
    let schema = explicit.or_else(|| envelope.get("version"));
    // Header-only legacy fixtures predate the explicit field; retain their
    // established version-1 reader behaviour until they are rewritten.
    let Some(schema) = schema.and_then(serde_json::Value::as_u64) else {
        return Ok(());
    };
    if schema != CAPTION_ARCHIVE_SCHEMA_VERSION as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unsupported caption archive schema version {schema}"),
        ));
    }
    Ok(())
}

fn interval_bounds(value: &serde_json::Value) -> Option<(i64, i64)> {
    let begin = ["begin_ms", "start_ms", "beginMs", "startMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))?;
    let end = ["end_ms", "endMs"]
        .iter()
        .find_map(|key| value.get(*key).and_then(serde_json::Value::as_i64))
        .unwrap_or_else(|| begin.saturating_add(5_000));
    (end > begin).then_some((begin, end))
}

fn scene_bounds(value: &serde_json::Value, kind: Option<&str>) -> Option<(i64, i64)> {
    if kind != Some("scene") {
        return None;
    }
    let begin = value.get("pts_ms").and_then(serde_json::Value::as_i64)?;
    let wait = value
        .get("wait_duration_ms")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(5_000);
    let end = if wait > 0 && wait < i64::MAX / 2 {
        begin.saturating_add(wait)
    } else {
        i64::MAX
    };
    (end > begin).then_some((begin, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_only_intervals_active_at_requested_time() {
        let path = std::env::temp_dir().join(format!(
            "arib-caption-archive-preview-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            concat!(
                "{\"type\":\"header\"}\n",
                "{\"type\":\"region_interval\",\"value\":{\"begin_ms\":100,\"end_ms\":500,\"text\":\"A\"}}\n",
                "{\"type\":\"region_interval\",\"value\":{\"begin_ms\":300,\"end_ms\":900,\"text\":\"B\"}}\n",
                "{\"type\":\"region_interval\",\"value\":{\"begin_ms\":1200,\"end_ms\":1500,\"text\":\"C\"}}\n",
            ),
        )
        .expect("archive fixture");
        let preview = render_archive_at(&path, 400).expect("render preview");
        assert_eq!(preview.intervals.len(), 2);
        assert_eq!(preview.intervals[0]["text"], "A");
        assert_eq!(preview.intervals[1]["text"], "B");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_an_unsupported_archive_schema() {
        let path = std::env::temp_dir().join(format!(
            "arib-caption-archive-schema-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "{\"type\":\"arib_caption_studio_archive\",\"schemaVersion\":99}\n",
        )
        .expect("archive fixture");
        let error = render_archive_at(&path, 0).expect_err("unsupported schema");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_conflicting_archive_schema_aliases() {
        let path = std::env::temp_dir().join(format!(
            "arib-caption-archive-schema-conflict-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "{\"type\":\"arib_caption_studio_archive\",\"schemaVersion\":1,\"version\":2}\n",
        )
        .expect("archive fixture");
        let error = render_archive_at(&path, 0).expect_err("conflicting schema aliases");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn scene_snapshots_replace_older_scenes_and_interval_records() {
        let path = std::env::temp_dir().join(format!(
            "arib-caption-scene-replacement-{}.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            concat!(
                r#"{"type":"scene","value":{"pts_ms":100,"wait_duration_ms":9223372036854775807,"text":"old"}}"#,
                "\n",
                r#"{"type":"region_interval","value":{"begin_ms":100,"end_ms":900,"text":"interval"}}"#,
                "\n",
                r#"{"type":"scene","value":{"pts_ms":400,"wait_duration_ms":9223372036854775807,"text":"new"}}"#,
            ),
        )
        .expect("archive fixture");
        let preview = render_archive_at(&path, 600).expect("render preview");
        assert_eq!(preview.intervals.len(), 1);
        assert_eq!(preview.intervals[0]["text"], "new");
        let _ = std::fs::remove_file(path);
    }
}
