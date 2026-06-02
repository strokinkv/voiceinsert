use crate::paths::AppPaths;
use std::time::{Duration, SystemTime};

pub fn init(paths: &AppPaths, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(paths.logs_dir())?;
    cleanup_old_logs(paths.logs_dir().as_path())?;
    let filter = match level {
        "Debug" => "debug",
        "Warning" => "warn",
        "Error" => "error",
        _ => "info",
    };
    let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "voiceinsert.log");
    if let Err(err) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .try_init()
    {
        eprintln!("warning: failed to initialize logging: {err}");
    }

    Ok(())
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
    use super::should_delete_log_file;
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
}
