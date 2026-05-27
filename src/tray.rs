use crate::i18n;
use crate::settings::AppLanguage;

const SETTINGS_MENU_ID: &str = "voiceinsert-settings";
const EXIT_MENU_ID: &str = "voiceinsert-exit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Recording,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Settings,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuLabels {
    pub settings: String,
    pub exit: String,
}

impl TrayMenuLabels {
    pub fn for_language(language: AppLanguage) -> Self {
        let texts = i18n::texts(language);
        Self {
            settings: texts.settings.to_string(),
            exit: texts.exit.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct TrayService {
    labels: TrayMenuLabels,
    state: TrayState,
}

impl TrayService {
    pub fn new(language: AppLanguage) -> Self {
        Self {
            labels: TrayMenuLabels::for_language(language),
            state: TrayState::Idle,
        }
    }

    pub fn labels(&self) -> &TrayMenuLabels {
        &self.labels
    }

    pub fn state(&self) -> TrayState {
        self.state
    }

    pub fn set_language(&mut self, language: AppLanguage) {
        self.labels = TrayMenuLabels::for_language(language);
    }

    pub fn set_state(&mut self, state: TrayState) {
        self.state = state;
    }
}

pub struct RuntimeTray {
    _tray_icon: tray_icon::TrayIcon,
    _menu: tray_icon::menu::Menu,
    settings_item: tray_icon::menu::MenuItem,
    exit_item: tray_icon::menu::MenuItem,
    service: TrayService,
}

impl RuntimeTray {
    pub fn new(language: AppLanguage) -> anyhow::Result<Self> {
        let service = TrayService::new(language);
        let menu = tray_icon::menu::Menu::new();
        let settings_item = tray_icon::menu::MenuItem::with_id(
            SETTINGS_MENU_ID,
            service.labels().settings.as_str(),
            true,
            None,
        );
        let exit_item = tray_icon::menu::MenuItem::with_id(
            EXIT_MENU_ID,
            service.labels().exit.as_str(),
            true,
            None,
        );

        menu.append(&settings_item)?;
        menu.append(&tray_icon::menu::PredefinedMenuItem::separator())?;
        menu.append(&exit_item)?;

        let tray_icon = tray_icon::TrayIconBuilder::new()
            .with_tooltip("VoiceInsert")
            .with_menu(Box::new(menu.clone()))
            .with_icon(default_icon()?)
            .build()?;

        Ok(Self {
            _tray_icon: tray_icon,
            _menu: menu,
            settings_item,
            exit_item,
            service,
        })
    }

    pub fn next_command(&mut self) -> Option<TrayCommand> {
        let event = tray_icon::menu::MenuEvent::receiver().try_recv().ok()?;
        if event.id == self.settings_item.id() {
            Some(TrayCommand::Settings)
        } else if event.id == self.exit_item.id() {
            Some(TrayCommand::Exit)
        } else {
            None
        }
    }

    pub fn set_language(&mut self, language: AppLanguage) {
        self.service.set_language(language);
        self.settings_item
            .set_text(self.service.labels().settings.as_str());
        self.exit_item.set_text(self.service.labels().exit.as_str());
    }

    pub fn set_state(&mut self, state: TrayState) {
        self.service.set_state(state);
    }

    pub fn state(&self) -> TrayState {
        self.service.state()
    }
}

fn default_icon() -> anyhow::Result<tray_icon::Icon> {
    #[cfg(windows)]
    {
        if let Ok(exe_path) = std::env::current_exe() {
            let installed_icon = exe_path.with_file_name("VoiceInsert.ico");
            if installed_icon.exists() {
                return Ok(tray_icon::Icon::from_path(installed_icon, Some((32, 32)))?);
            }
        }

        let source_icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("VoiceInsert.ico");
        if source_icon.exists() {
            return Ok(tray_icon::Icon::from_path(source_icon, Some((32, 32)))?);
        }
    }

    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as i32 - 15;
            let dy = y as i32 - 15;
            let inside = dx * dx + dy * dy <= 14 * 14;
            if inside {
                rgba.extend_from_slice(&[0x51, 0xb3, 0xa2, 0xff]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    Ok(tray_icon::Icon::from_rgba(rgba, SIZE, SIZE)?)
}

#[cfg(test)]
mod tests {
    use super::TrayMenuLabels;
    use crate::settings::AppLanguage;

    #[test]
    fn tray_labels_are_localized() {
        let ru = TrayMenuLabels::for_language(AppLanguage::Russian);
        let en = TrayMenuLabels::for_language(AppLanguage::English);

        assert_eq!(ru.settings, "Настройки");
        assert_eq!(ru.exit, "Выход");
        assert_eq!(en.settings, "Settings");
        assert_eq!(en.exit, "Exit");
    }
}
