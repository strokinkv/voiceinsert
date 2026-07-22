use crate::paths::AppPaths;
use std::time::{Duration, SystemTime};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub fn init(paths: &AppPaths, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(paths.logs_dir())?;
    migrate_legacy_log_names(paths.logs_dir().as_path())?;
    cleanup_old_logs(paths.logs_dir().as_path())?;
    let filter = match level {
        "Debug" => "debug",
        "Warning" => "warn",
        "Error" => "error",
        _ => "info",
    };
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("voiceinsert")
        .filename_suffix("log")
        .build(paths.logs_dir())?;
    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .try_init()
    {
        eprintln!("warning: failed to initialize logging: {err}");
    }

    Ok(())
}

fn migrate_legacy_log_names(logs_dir: &std::path::Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(logs_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(target_name) = legacy_log_target_name(file_name) else {
            continue;
        };

        let target = available_log_path(logs_dir, &target_name);
        let _ = std::fs::rename(entry.path(), target);
    }

    Ok(())
}

fn legacy_log_target_name(file_name: &str) -> Option<String> {
    let date = file_name.strip_prefix("voiceinsert.log.")?;
    if date.is_empty() {
        return None;
    }
    Some(format!("voiceinsert.{date}.log"))
}

fn available_log_path(logs_dir: &std::path::Path, file_name: &str) -> std::path::PathBuf {
    let first = logs_dir.join(file_name);
    if !first.exists() {
        return first;
    }

    let stem = file_name.strip_suffix(".log").unwrap_or(file_name);
    for suffix in 1..100 {
        let candidate = logs_dir.join(format!("{stem}.legacy-{suffix}.log"));
        if !candidate.exists() {
            return candidate;
        }
    }

    logs_dir.join(format!("{stem}.legacy.log"))
}

fn cleanup_old_logs(logs_dir: &std::path::Path) -> anyhow::Result<()> {
    let now = SystemTime::now();
    let retention = Duration::from_secs(7 * 24 * 60 * 60);

    for entry in std::fs::read_dir(logs_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(_) => continue,
        };

        if should_delete_log_file(modified, now, retention) {
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(())
}

fn should_delete_log_file(modified: SystemTime, now: SystemTime, retention: Duration) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age > retention)
}

#[cfg(test)]
mod tests {
    use super::{available_log_path, legacy_log_target_name, should_delete_log_file};
    use std::time::{Duration, SystemTime};

    #[test]
    fn delete_old_logs_after_retention_window() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let modified = now - Duration::from_secs(8 * 24 * 60 * 60);

        assert!(should_delete_log_file(
            modified,
            now,
            Duration::from_secs(7 * 24 * 60 * 60)
        ));
    }

    #[test]
    fn keep_recent_logs_within_retention_window() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let modified = now - Duration::from_secs(6 * 24 * 60 * 60);

        assert!(!should_delete_log_file(
            modified,
            now,
            Duration::from_secs(7 * 24 * 60 * 60)
        ));
    }

    #[test]
    fn legacy_log_names_are_migrated_to_log_suffix() {
        assert_eq!(
            legacy_log_target_name("voiceinsert.log.2026-07-22").as_deref(),
            Some("voiceinsert.2026-07-22.log")
        );
        assert_eq!(legacy_log_target_name("voiceinsert.2026-07-22.log"), None);
    }

    #[test]
    fn existing_log_target_uses_legacy_suffix() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("voiceinsert.2026-07-22.log"), "").unwrap();

        let target = available_log_path(temp.path(), "voiceinsert.2026-07-22.log");

        assert_eq!(
            target.file_name().and_then(|name| name.to_str()),
            Some("voiceinsert.2026-07-22.legacy-1.log")
        );
    }
}
