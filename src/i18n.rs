use crate::settings::AppLanguage;

#[derive(Debug, Clone, Copy)]
pub struct Texts {
    pub settings: &'static str,
    pub exit: &'static str,
    pub recording: &'static str,
    pub transcribing: &'static str,
    pub inserting: &'static str,
    pub error: &'static str,
}

pub fn texts(language: AppLanguage) -> Texts {
    match language {
        AppLanguage::Russian => Texts {
            settings: "Настройки",
            exit: "Выход",
            recording: "Запись",
            transcribing: "Распознавание",
            inserting: "Вставка",
            error: "Ошибка",
        },
        AppLanguage::English => Texts {
            settings: "Settings",
            exit: "Exit",
            recording: "Recording",
            transcribing: "Transcribing",
            inserting: "Inserting",
            error: "Error",
        },
    }
}
