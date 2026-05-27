use crate::i18n;
use crate::settings::AppLanguage;

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
