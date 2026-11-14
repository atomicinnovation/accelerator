use std::path::Path;

use file_rotate::compression::Compression;
use file_rotate::suffix::AppendCount;
use file_rotate::{ContentLimit, FileRotate};
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("failed to create log directory {path}: {source}")]
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("logging subscriber already installed")]
    AlreadyInitialised,
}

pub(crate) const MAX_BYTES: usize = 5 * 1024 * 1024;
pub(crate) const MAX_FILES: usize = 3;

pub fn make_writer(
    path: &Path,
) -> Result<FileRotate<AppendCount>, LoggingError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            LoggingError::CreateDir {
                path: parent.to_path_buf(),
                source: e,
            }
        })?;
    }
    Ok(FileRotate::new(
        path,
        AppendCount::new(MAX_FILES),
        ContentLimit::Bytes(MAX_BYTES),
        Compression::None,
        #[cfg(unix)]
        Some(0o600),
        #[cfg(not(unix))]
        None,
    ))
}

pub fn init(log_path: &Path) -> Result<WorkerGuard, LoggingError> {
    let writer = make_writer(log_path)?;
    let (nb, guard) = tracing_appender::non_blocking(writer);
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(nb)
        .try_init()
        .map_err(|_| LoggingError::AlreadyInitialised)?;
    Ok(guard)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Arc, Mutex, MutexGuard};

    use tracing_subscriber::fmt::MakeWriter;

    pub struct MutexWriter<W: Write + Send + 'static>(Arc<Mutex<W>>);

    impl<W: Write + Send + 'static> MutexWriter<W> {
        pub fn new(w: W) -> Self {
            Self(Arc::new(Mutex::new(w)))
        }
    }

    pub struct MutexGuardWriter<'a, W: Write + Send + 'static>(
        MutexGuard<'a, W>,
    );

    impl<W: Write + Send + 'static> Write for MutexGuardWriter<'_, W> {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }

    impl<'a, W: Write + Send + 'static> MakeWriter<'a> for MutexWriter<W> {
        type Writer = MutexGuardWriter<'a, W>;
        fn make_writer(&'a self) -> Self::Writer {
            MutexGuardWriter(self.0.lock().expect("MutexWriter poisoned"))
        }
    }

    pub fn build_test_json_subscriber(
        path: &std::path::Path,
    ) -> impl tracing::Subscriber + Send + Sync {
        let writer = super::make_writer(path).expect("make_writer");
        tracing_subscriber::fmt()
            .json()
            .with_writer(MutexWriter::new(writer))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    const ROTATION_CAP: u64 = 5 * 1024 * 1024;
    const RETAINED_SEGMENTS: usize = 3;
    const OVERSHOOT: u64 = 64 * 1024;

    fn write_past_the_cap(
        writer: &mut FileRotate<AppendCount>,
        rotations: usize,
    ) {
        let past_the_cap = vec![b'x'; (ROTATION_CAP + OVERSHOOT) as usize];
        for _ in 0..rotations {
            writer.write_all(&past_the_cap).unwrap();
        }
        writer.flush().unwrap();
    }

    fn log_segments(dir: &Path) -> Vec<std::fs::DirEntry> {
        let mut segments: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("server.log")
            })
            .collect();
        segments.sort_by_key(std::fs::DirEntry::file_name);
        segments
    }

    fn rotated_segments(dir: &Path) -> Vec<std::fs::DirEntry> {
        log_segments(dir)
            .into_iter()
            .filter(|e| e.file_name() != "server.log")
            .collect()
    }

    fn rotated_segment_sizes(dir: &Path) -> Vec<u64> {
        rotated_segments(dir)
            .iter()
            .map(|e| e.metadata().unwrap().len())
            .collect()
    }

    #[test]
    fn rotates_when_active_segment_crosses_5_megabytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();

        write_past_the_cap(&mut writer, 1);
        drop(writer);

        assert_eq!(rotated_segment_sizes(dir.path()), vec![ROTATION_CAP]);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), OVERSHOOT);
    }

    #[test]
    fn retains_at_most_three_rotated_segments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();

        write_past_the_cap(&mut writer, 5);
        drop(writer);

        assert_eq!(
            rotated_segment_sizes(dir.path()),
            vec![ROTATION_CAP; RETAINED_SEGMENTS]
        );
    }

    #[cfg(unix)]
    #[test]
    fn active_and_rotated_segments_are_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let mut writer = make_writer(&path).unwrap();
        let rotations = 2;

        write_past_the_cap(&mut writer, rotations);
        drop(writer);

        let segments = log_segments(dir.path());
        assert_eq!(segments.len(), rotations + 1);
        for segment in segments {
            let mode = segment.metadata().unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "{:?} must be owner-only",
                segment.file_name()
            );
        }
    }

    #[test]
    fn make_writer_errors_when_parent_path_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let bad_path = blocker.join("server.log");
        let result = make_writer(&bad_path);
        assert!(result.is_err(), "expected an error");
        let err = result.err().unwrap();
        assert!(
            matches!(err, LoggingError::CreateDir { .. }),
            "expected CreateDir error, got {err:?}",
        );
    }

    #[test]
    fn pre_existing_oversized_log_rotates_on_first_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        std::fs::write(&path, vec![b'x'; MAX_BYTES + 1024]).unwrap();
        let mut writer = make_writer(&path).unwrap();
        writer.write_all(b"trigger\n").unwrap();
        writer.flush().unwrap();
        drop(writer);
        assert!(!rotated_segments(dir.path()).is_empty());
    }

    #[test]
    fn emits_json_lines_with_message_and_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log");
        let subscriber = test_support::build_test_json_subscriber(&path);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(field = "value", "hello");
        });

        let body = std::fs::read_to_string(&path).unwrap();
        let line = body.lines().next().expect("at least one log line");
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v["fields"]["message"], "hello");
        assert_eq!(v["fields"]["field"], "value");
    }
}
