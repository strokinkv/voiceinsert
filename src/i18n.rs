use crate::settings::AppLanguage;

#[derive(Debug, Clone, Copy)]
pub struct Texts {
    pub settings: &'static str,
    pub exit: &'static str,
    pub recording: &'static str,
    pub transcribing: &'static str,
    pub inserting: &'static str,
    pub error: &'static str,
    pub settings_saved: &'static str,
    pub models_loading: &'static str,
    pub models_loaded: &'static str,
    pub api_profile_added: &'static str,
    pub api_profile_deleted: &'static str,
    pub api_profile_selected: &'static str,
    pub api_profile_required: &'static str,
    pub no_speech_detected: &'static str,
    pub transcription_cancelled: &'static str,
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
            settings_saved: "Настройки сохранены.",
            models_loading: "Загрузка моделей...",
            models_loaded: "Модели загружены.",
            api_profile_added: "API профиль добавлен.",
            api_profile_deleted: "API профиль удалён.",
            api_profile_selected: "API профиль выбран.",
            api_profile_required: "Нужен хотя бы один API профиль.",
            no_speech_detected: "Речь не обнаружена.",
            transcription_cancelled: "Распознавание отменено.",
        },
        AppLanguage::English => Texts {
            settings: "Settings",
            exit: "Exit",
            recording: "Recording",
            transcribing: "Transcribing",
            inserting: "Inserting",
            error: "Error",
            settings_saved: "Settings saved.",
            models_loading: "Loading models...",
            models_loaded: "Models loaded.",
            api_profile_added: "API profile added.",
            api_profile_deleted: "API profile deleted.",
            api_profile_selected: "API profile selected.",
            api_profile_required: "At least one API profile is required.",
            no_speech_detected: "No speech detected.",
            transcription_cancelled: "Transcription cancelled.",
        },
    }
}
