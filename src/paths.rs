use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    app_data: PathBuf,
    local_data: PathBuf,
}

impl AppPaths {
    pub fn new() -> anyhow::Result<Self> {
        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("APPDATA is not set"))?
            .join("VoiceInsert");
        let local_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("LOCALAPPDATA is not set"))?
            .join("VoiceInsert");

        Ok(Self {
            app_data,
            local_data,
        })
    }

    pub fn for_test(root: &Path) -> Self {
        Self {
            app_data: root.join("appdata").join("VoiceInsert"),
            local_data: root.join("localappdata").join("VoiceInsert"),
        }
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.local_data.join("Logs")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.app_data.join("settings.json")
    }

    pub fn secrets_path(&self) -> PathBuf {
        self.app_data.join("api-key.dpapi")
    }
}
