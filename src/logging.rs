use crate::paths::AppPaths;

pub fn init(paths: &AppPaths, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(paths.logs_dir())?;
    let filter = match level {
        "Debug" => "debug",
        "Warning" => "warn",
        "Error" => "error",
        _ => "info",
    };
    let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "voiceinsert.log");
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .try_init();

    Ok(())
}
