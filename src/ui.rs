slint::include_modules!();

use crate::settings::AppSettings;
use slint::{ComponentHandle, SharedString};

pub struct UiController {
    settings_window: Option<SettingsWindow>,
}

impl UiController {
    pub fn new() -> Self {
        Self {
            settings_window: None,
        }
    }

    pub fn open_settings(&mut self, settings: &AppSettings) -> anyhow::Result<()> {
        let window = match &self.settings_window {
            Some(window) => window.clone_strong(),
            None => {
                let window = SettingsWindow::new()?;
                wire_settings_callbacks(&window);
                self.settings_window = Some(window.clone_strong());
                window
            }
        };

        let profile = settings.active_profile();
        window.set_status_text(SharedString::from(format!(
            "Profile: {} ({})",
            profile.name, profile.base_url
        )));
        window.show()?;
        Ok(())
    }
}

impl Default for UiController {
    fn default() -> Self {
        Self::new()
    }
}

fn wire_settings_callbacks(window: &SettingsWindow) {
    let weak = window.as_weak();
    window.on_close(move || {
        if let Some(window) = weak.upgrade() {
            let _ = window.hide();
        }
    });

    let weak = window.as_weak();
    window.on_save(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status_text(SharedString::from("Settings saved."));
        }
    });
}
