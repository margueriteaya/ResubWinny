use std::{fs, io, path::Path};

/// Publishes a complete replacement in the target directory. Windows does not
/// allow `rename(part, existing)` directly, so retain the old file until the
/// replacement has been installed successfully.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let part = path.with_extension(format!("{extension}.part"));
    let backup = path.with_extension(format!("{extension}.backup"));

    fs::write(&part, bytes)?;
    let part_file = fs::OpenOptions::new().write(true).open(&part)?;
    part_file.sync_all()?;
    drop(part_file);
    if !path.exists() {
        return fs::rename(part, path);
    }
    if backup.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "existing backup blocks atomic publish: {}",
                backup.display()
            ),
        ));
    }
    fs::rename(path, &backup).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "could not retain current file as {}: {error}",
                backup.display()
            ),
        )
    })?;
    if let Err(error) = fs::rename(&part, path) {
        let _ = fs::rename(&backup, path);
        return Err(io::Error::new(
            error.kind(),
            format!("could not publish replacement {}: {error}", path.display()),
        ));
    }
    fs::remove_file(&backup).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "replacement was published but backup cleanup failed {}: {error}",
                backup.display()
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::write_atomic;

    #[test]
    fn replaces_existing_json_without_leaving_a_backup() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resubwinny-storage-{stamp}.json"));
        std::fs::write(&path, b"old").expect("old value");
        write_atomic(&path, b"new").expect("replacement");
        assert_eq!(std::fs::read(&path).expect("published value"), b"new");
        assert!(!path.with_extension("json.part").exists());
        assert!(!path.with_extension("json.backup").exists());
        std::fs::remove_file(path).expect("cleanup");
    }
}
