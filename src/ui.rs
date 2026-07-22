slint::include_modules!();

use crate::border_indicator::{BorderIndicator, BorderIndicatorColor};
use crate::overlay::{overlay_position, primary_work_area};
use crate::settings::{AI2NPU_DEFAULT_MODEL, AppLanguage, AppSettings, RecordingMode};
use slint::{
    CloseRequestResponse, ComponentHandle, ModelRc, PhysicalPosition, SharedString, VecModel,
};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    SettingsChanged,
    AddApiProfile,
    DeleteApiProfile,
    SelectApiProfile(String),
    OpenLogsFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsEdit {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: String,
    pub request_timeout_seconds: String,
    pub transcription_hotkey: String,
    pub recording_mode: String,
    pub silence_threshold_percent: String,
    pub silence_timeout_milliseconds: String,
    pub max_recording_seconds: String,
    pub start_with_windows: bool,
    pub ui_language: String,
    pub enable_sounds: bool,
    pub restore_clipboard_content: bool,
    pub delay_before_paste_milliseconds: String,
    pub delay_before_clipboard_restore_milliseconds: String,
    pub log_level: String,
}

pub struct UiController {
    settings_window: Option<SettingsWindow>,
    recording_overlay: Option<RecordingOverlay>,
    border_indicator: BorderIndicator,
    command_tx: Sender<UiCommand>,
    command_rx: Receiver<UiCommand>,
    overlay_wave_step: i32,
    overlay_wave_tick: u8,
    overlay_level: f32,
    overlay_wave_levels: [f32; OVERLAY_WAVE_BARS],
}

const OVERLAY_WAVE_BARS: usize = 14;

impl UiController {
    pub fn new() -> Self {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        Self {
            settings_window: None,
            recording_overlay: None,
            border_indicator: BorderIndicator::new(),
            command_tx,
            command_rx,
            overlay_wave_step: 0,
            overlay_wave_tick: 0,
            overlay_level: 0.0,
            overlay_wave_levels: [0.0; OVERLAY_WAVE_BARS],
        }
    }

    pub fn open_settings(
        &mut self,
        settings: &AppSettings,
        api_key: &str,
        model_options: &[String],
    ) -> anyhow::Result<()> {
        let window = match &self.settings_window {
            Some(window) => window.clone_strong(),
            None => {
                let window = SettingsWindow::new()?;
                wire_settings_callbacks(&window, self.command_tx.clone());
                self.settings_window = Some(window.clone_strong());
                window
            }
        };

        let profile = settings.active_profile();
        let model_name = model_label(&profile.model).to_string();
        let profile_options = api_profile_options_for_display(settings);
        window.set_api_profile_options(shared_string_model(&profile_options));
        window.set_active_profile_index(0);
        window.set_active_profile_name(SharedString::from(profile.name.as_str()));
        window.set_api_base_url(SharedString::from(profile.base_url.as_str()));
        window.set_api_key(SharedString::from(api_key));
        window.set_model_options(shared_string_model(&model_options_for_display(
            &model_name,
            model_options,
        )));
        window.set_model_index(0);
        window.set_model_name(SharedString::from(model_name.as_str()));
        window.set_temperature(SharedString::from(format!("{:.1}", profile.temperature)));
        window.set_request_timeout_seconds(SharedString::from(
            profile.request_timeout_seconds.to_string(),
        ));
        window.set_transcription_hotkey(SharedString::from(settings.hotkey.as_str()));
        window.set_recording_mode_options(shared_string_model(
            &recording_mode_options_for_display(settings.recording_mode, settings.ui_language),
        ));
        window.set_recording_mode_index(0);
        window.set_recording_mode(SharedString::from(recording_mode_display_label(
            settings.recording_mode,
            settings.ui_language,
        )));
        window.set_silence_threshold_percent(SharedString::from(
            settings.silence_threshold_percent.to_string(),
        ));
        window.set_silence_timeout_milliseconds(SharedString::from(
            settings.silence_timeout_milliseconds.to_string(),
        ));
        window.set_max_recording_seconds(SharedString::from(
            settings.max_recording_seconds.to_string(),
        ));
        window.set_start_with_windows(settings.start_with_windows);
        window.set_ui_language_options(shared_string_model(&language_options_for_display(
            settings.ui_language,
        )));
        window.set_ui_language_index(0);
        window.set_ui_language(SharedString::from(language_label(settings.ui_language)));
        window.set_enable_sounds(settings.enable_sounds);
        window.set_restore_clipboard_content(settings.restore_clipboard_content);
        window.set_delay_before_paste_milliseconds(SharedString::from(
            settings.delay_before_paste_milliseconds.to_string(),
        ));
        window.set_delay_before_clipboard_restore_milliseconds(SharedString::from(
            settings
                .delay_before_clipboard_restore_milliseconds
                .to_string(),
        ));
        window.set_log_level_options(shared_string_model(&log_level_options_for_display(
            &settings.log_level,
            settings.ui_language,
        )));
        window.set_log_level_index(0);
        window.set_log_level(SharedString::from(log_level_display_label(
            &settings.log_level,
            settings.ui_language,
        )));
        window.set_status_text(SharedString::from(""));
        window.show()?;
        disable_settings_window_resize();
        Ok(())
    }

    pub fn show_recording_overlay(&mut self, _status: &str) -> anyhow::Result<()> {
        self.border_indicator
            .show(BorderIndicatorColor::Recording)?;
        Ok(())
    }

    pub fn show_legacy_recording_overlay(&mut self, status: &str) -> anyhow::Result<()> {
        let overlay = self.overlay()?;
        overlay.set_status_text(SharedString::from(status));
        overlay.set_wave_step(0);
        self.overlay_wave_step = 0;
        self.overlay_wave_tick = 0;
        self.overlay_level = 0.0;
        self.overlay_wave_levels = [0.0; OVERLAY_WAVE_BARS];
        set_overlay_wave_levels(&overlay, &self.overlay_wave_levels);
        position_overlay_near_clock(&overlay);
        overlay.show()?;
        Ok(())
    }

    pub fn set_overlay_level(&mut self, level: f32) {
        if self.recording_overlay.is_some() {
            self.overlay_level = level.clamp(0.0, 1.0);
        }
    }

    pub fn set_overlay_status(&mut self, _status: &str) -> anyhow::Result<()> {
        self.border_indicator
            .show(BorderIndicatorColor::Transcribing)?;
        Ok(())
    }

    pub fn set_legacy_overlay_status(&mut self, status: &str) -> anyhow::Result<()> {
        let overlay = self.overlay()?;
        overlay.set_status_text(SharedString::from(status));
        position_overlay_near_clock(&overlay);
        overlay.show()?;
        Ok(())
    }

    pub fn hide_overlay(&mut self) {
        self.border_indicator.hide();
        if let Some(overlay) = &self.recording_overlay {
            let _ = overlay.hide();
        }
    }

    pub fn advance_overlay_wave(&mut self) {
        if let Some(overlay) = &self.recording_overlay {
            self.overlay_wave_tick = self.overlay_wave_tick.wrapping_add(1);
            if self.overlay_wave_tick.is_multiple_of(8) {
                self.overlay_wave_step = (self.overlay_wave_step + 1) % 14;
                self.overlay_wave_levels.rotate_left(1);
                self.overlay_wave_levels[OVERLAY_WAVE_BARS - 1] = self.overlay_level;
                overlay.set_wave_step(self.overlay_wave_step);
                set_overlay_wave_levels(overlay, &self.overlay_wave_levels);
            }
        }
    }

    pub fn set_status(&self, message: impl Into<SharedString>) {
        if let Some(window) = &self.settings_window {
            window.set_status_text(message.into());
        }
    }

    pub fn capture_copilot_hotkey(&self) -> bool {
        let Some(window) = &self.settings_window else {
            return false;
        };
        if !window.get_capturing_transcription_hotkey() {
            return false;
        }

        window.set_capturing_transcription_hotkey(false);
        window.set_transcription_hotkey(SharedString::from("Shift+Win+F23"));
        window.set_status_text(SharedString::from(""));
        let _ = self.command_tx.send(UiCommand::SettingsChanged);
        true
    }

    pub fn set_profile_metadata(&self, settings: &AppSettings) {
        if let Some(window) = &self.settings_window {
            let profile = settings.active_profile();
            window.set_api_profile_options(shared_string_model(&api_profile_options_for_display(
                settings,
            )));
            window.set_active_profile_index(0);
            window.set_active_profile_name(SharedString::from(profile.name.as_str()));
        }
    }

    pub fn set_model_options(&self, current_model: &str, model_options: &[String]) {
        if let Some(window) = &self.settings_window {
            let visible_model = window.get_model_name().to_string();
            let current_model = if visible_model.trim().is_empty() {
                current_model
            } else {
                visible_model.trim()
            };
            window.set_model_options(shared_string_model(&model_options_for_display(
                current_model,
                model_options,
            )));
            window.set_model_index(0);
            window.set_model_name(SharedString::from(current_model));
        }
    }

    pub fn drain_commands(&self) -> Vec<UiCommand> {
        let mut commands = Vec::new();
        loop {
            match self.command_rx.try_recv() {
                Ok(command) => commands.push(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        commands
    }

    pub fn settings_edit(&self) -> Option<SettingsEdit> {
        let window = self.settings_window.as_ref()?;
        Some(SettingsEdit {
            base_url: window.get_api_base_url().to_string(),
            api_key: window.get_api_key().to_string(),
            model: window.get_model_name().to_string(),
            temperature: window.get_temperature().to_string(),
            request_timeout_seconds: window.get_request_timeout_seconds().to_string(),
            transcription_hotkey: window.get_transcription_hotkey().to_string(),
            recording_mode: window.get_recording_mode().to_string(),
            silence_threshold_percent: window.get_silence_threshold_percent().to_string(),
            silence_timeout_milliseconds: window.get_silence_timeout_milliseconds().to_string(),
            max_recording_seconds: window.get_max_recording_seconds().to_string(),
            start_with_windows: window.get_start_with_windows(),
            ui_language: window.get_ui_language().to_string(),
            enable_sounds: window.get_enable_sounds(),
            restore_clipboard_content: window.get_restore_clipboard_content(),
            delay_before_paste_milliseconds: window
                .get_delay_before_paste_milliseconds()
                .to_string(),
            delay_before_clipboard_restore_milliseconds: window
                .get_delay_before_clipboard_restore_milliseconds()
                .to_string(),
            log_level: window.get_log_level().to_string(),
        })
    }

    fn overlay(&mut self) -> anyhow::Result<RecordingOverlay> {
        match &self.recording_overlay {
            Some(overlay) => Ok(overlay.clone_strong()),
            None => {
                let overlay = RecordingOverlay::new()?;
                self.recording_overlay = Some(overlay.clone_strong());
                Ok(overlay)
            }
        }
    }
}

fn set_overlay_wave_levels(overlay: &RecordingOverlay, levels: &[f32; OVERLAY_WAVE_BARS]) {
    overlay.set_wave_a(levels[0]);
    overlay.set_wave_b(levels[1]);
    overlay.set_wave_c(levels[2]);
    overlay.set_wave_d(levels[3]);
    overlay.set_wave_e(levels[4]);
    overlay.set_wave_f(levels[5]);
    overlay.set_wave_g(levels[6]);
    overlay.set_wave_h(levels[7]);
    overlay.set_wave_i(levels[8]);
    overlay.set_wave_j(levels[9]);
    overlay.set_wave_k(levels[10]);
    overlay.set_wave_l(levels[11]);
    overlay.set_wave_m(levels[12]);
    overlay.set_wave_n(levels[13]);
}

fn position_overlay_near_clock(overlay: &RecordingOverlay) {
    let width = overlay.window().size().width as i32;
    let height = overlay.window().size().height as i32;
    let work_area = primary_work_area();
    let (x, y) = overlay_position(work_area, (width, height), 24);
    overlay.window().set_position(PhysicalPosition::new(x, y));
}

#[cfg(windows)]
fn disable_settings_window_resize() {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_STYLE, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WINDOW_STYLE,
        WS_MAXIMIZEBOX, WS_THICKFRAME,
    };
    use windows::core::BOOL;

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = unsafe { &mut *(lparam.0 as *mut Option<HWND>) };
        if data.is_some() || !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return true.into();
        }

        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
        if process_id != unsafe { GetCurrentProcessId() } {
            return true.into();
        }

        let title_len = unsafe { GetWindowTextLengthW(hwnd) };
        if title_len <= 0 {
            return true.into();
        }

        let mut title = vec![0u16; title_len as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut title) };
        if copied <= 0 {
            return true.into();
        }

        let title = String::from_utf16_lossy(&title[..copied as usize]);
        if title == "VoiceInsert" {
            *data = Some(hwnd);
        }
        true.into()
    }

    unsafe {
        let mut hwnd = None;
        let _ = EnumWindows(Some(enum_window), LPARAM(&mut hwnd as *mut _ as isize));
        if let Some(hwnd) = hwnd {
            let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
            let style = style & !WS_THICKFRAME & !WS_MAXIMIZEBOX;
            SetWindowLongPtrW(hwnd, GWL_STYLE, style.0 as isize);
            let flags: SET_WINDOW_POS_FLAGS =
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED;
            let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, flags);
        }
    }
}

#[cfg(not(windows))]
fn disable_settings_window_resize() {}

fn model_label(model: &str) -> &str {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        AI2NPU_DEFAULT_MODEL
    } else {
        trimmed
    }
}

fn model_options_for_display(current_model: &str, options: &[String]) -> Vec<String> {
    let mut values = Vec::new();
    let current_model = model_label(current_model).to_string();
    if !current_model.trim().is_empty() {
        values.push(current_model.clone());
    }

    for option in options {
        let trimmed = option.trim();
        if !trimmed.is_empty() && !values.iter().any(|value| value == trimmed) {
            values.push(trimmed.to_string());
        }
    }

    values
}

fn api_profile_options_for_display(settings: &AppSettings) -> Vec<String> {
    let active_profile = settings.active_profile();
    let mut values = vec![active_profile.name.clone()];
    for profile in &settings.api_profiles {
        if !profile.name.trim().is_empty() && !values.iter().any(|value| value == &profile.name) {
            values.push(profile.name.clone());
        }
    }
    values
}

fn recording_mode_options_for_display(mode: RecordingMode, language: AppLanguage) -> Vec<String> {
    current_first(
        recording_mode_display_label(mode, language),
        match language {
            AppLanguage::Russian => &["Переключатель", "Удержание", "Тишина"],
            AppLanguage::English => &["Toggle", "Hold", "SilenceTimeout"],
        },
    )
}

fn log_level_options_for_display(level: &str, language: AppLanguage) -> Vec<String> {
    current_first(
        log_level_display_label(level, language),
        match language {
            AppLanguage::Russian => &["Инфо", "Отладка", "Предупреждения", "Ошибки"],
            AppLanguage::English => &["Information", "Debug", "Warning", "Error"],
        },
    )
}

fn language_options_for_display(language: AppLanguage) -> Vec<String> {
    current_first(language_label(language), &["Русский", "English"])
}

fn current_first(current: &str, options: &[&str]) -> Vec<String> {
    let mut values = vec![current.to_string()];
    for option in options {
        if !values.iter().any(|value| value == option) {
            values.push((*option).to_string());
        }
    }
    values
}

fn shared_string_model(values: &[String]) -> ModelRc<SharedString> {
    let rows = values
        .iter()
        .map(|value| SharedString::from(value.as_str()))
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn recording_mode_display_label(mode: RecordingMode, language: AppLanguage) -> &'static str {
    match (language, mode) {
        (AppLanguage::Russian, RecordingMode::Toggle) => "Переключатель",
        (AppLanguage::Russian, RecordingMode::Hold) => "Удержание",
        (AppLanguage::Russian, RecordingMode::SilenceTimeout) => "Тишина",
        (_, RecordingMode::Toggle) => "Toggle",
        (_, RecordingMode::Hold) => "Hold",
        (_, RecordingMode::SilenceTimeout) => "SilenceTimeout",
    }
}

fn log_level_display_label(level: &str, language: AppLanguage) -> &'static str {
    match (language, level) {
        (AppLanguage::Russian, "Debug") => "Отладка",
        (AppLanguage::Russian, "Warning") => "Предупреждения",
        (AppLanguage::Russian, "Error") => "Ошибки",
        (AppLanguage::Russian, _) => "Инфо",
        (_, "Debug") => "Debug",
        (_, "Warning") => "Warning",
        (_, "Error") => "Error",
        _ => "Information",
    }
}

fn language_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::Russian => "Русский",
        AppLanguage::English => "English",
    }
}

impl Default for UiController {
    fn default() -> Self {
        Self::new()
    }
}

fn wire_settings_callbacks(window: &SettingsWindow, command_tx: Sender<UiCommand>) {
    let weak_window = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(window) = weak_window.upgrade() {
            let _ = window.hide();
        }
        CloseRequestResponse::KeepWindowShown
    });

    let tx = command_tx.clone();
    window.on_settings_changed(move || {
        let _ = tx.send(UiCommand::SettingsChanged);
    });

    let tx = command_tx.clone();
    window.on_add_api_profile(move || {
        let _ = tx.send(UiCommand::AddApiProfile);
    });

    let tx = command_tx.clone();
    window.on_delete_api_profile(move || {
        let _ = tx.send(UiCommand::DeleteApiProfile);
    });

    let tx = command_tx.clone();
    window.on_select_api_profile(move |profile_name| {
        let _ = tx.send(UiCommand::SelectApiProfile(profile_name.to_string()));
    });

    let tx = command_tx.clone();
    window.on_open_logs_folder(move || {
        let _ = tx.send(UiCommand::OpenLogsFolder);
    });

    let tx = command_tx.clone();
    let weak_window = window.as_weak();
    window.on_capture_transcription_hotkey(move |key, control, alt, shift, meta| {
        let Some(window) = weak_window.upgrade() else {
            return false;
        };

        match captured_hotkey_label(&key, control, alt, shift, meta) {
            Ok(Some(hotkey)) => {
                window.set_transcription_hotkey(SharedString::from(hotkey));
                window.set_status_text(SharedString::from(""));
                let _ = tx.send(UiCommand::SettingsChanged);
                true
            }
            Ok(None) => false,
            Err(message) => {
                window.set_status_text(SharedString::from(message.to_string()));
                false
            }
        }
    });
}

fn captured_hotkey_label(
    key: &str,
    control: bool,
    alt: bool,
    shift: bool,
    meta: bool,
) -> anyhow::Result<Option<String>> {
    let Some(key) = captured_key_label(key) else {
        return Ok(None);
    };

    let mut parts = Vec::new();
    if control {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if meta {
        parts.push("Win");
    }
    parts.push(key.as_str());

    crate::hotkeys::matcher::normalize_hotkey(&parts.join("+"))
        .map(Some)
        .map_err(|_| anyhow::anyhow!("Неподдерживаемая горячая клавиша"))
}

fn captured_key_label(key: &str) -> Option<String> {
    let named_key = key.trim();
    if !named_key.is_empty() {
        if let Ok(hotkey) = crate::hotkeys::matcher::normalize_hotkey(named_key) {
            return Some(hotkey);
        }
    }

    let character = key.chars().next()?;
    if key.chars().count() != 1 {
        return None;
    }

    match character {
        '\u{0010}' | '\u{0011}' | '\u{0012}' | '\u{0013}' | '\u{0015}' | '\u{0016}'
        | '\u{0017}' | '\u{0018}' => None,
        '\u{0008}' => Some("Backspace".to_string()),
        '\u{0009}' => Some("Tab".to_string()),
        '\u{000a}' => Some("Enter".to_string()),
        '\u{001b}' => Some("Esc".to_string()),
        '\u{007f}' => Some("Delete".to_string()),
        ' ' => Some("Space".to_string()),
        '\u{F700}' => Some("Up".to_string()),
        '\u{F701}' => Some("Down".to_string()),
        '\u{F702}' => Some("Left".to_string()),
        '\u{F703}' => Some("Right".to_string()),
        '\u{F704}' => Some("F1".to_string()),
        '\u{F705}' => Some("F2".to_string()),
        '\u{F706}' => Some("F3".to_string()),
        '\u{F707}' => Some("F4".to_string()),
        '\u{F708}' => Some("F5".to_string()),
        '\u{F709}' => Some("F6".to_string()),
        '\u{F70A}' => Some("F7".to_string()),
        '\u{F70B}' => Some("F8".to_string()),
        '\u{F70C}' => Some("F9".to_string()),
        '\u{F70D}' => Some("F10".to_string()),
        '\u{F70E}' => Some("F11".to_string()),
        '\u{F70F}' => Some("F12".to_string()),
        '\u{F710}' => Some("F13".to_string()),
        '\u{F711}' => Some("F14".to_string()),
        '\u{F712}' => Some("F15".to_string()),
        '\u{F713}' => Some("F16".to_string()),
        '\u{F714}' => Some("F17".to_string()),
        '\u{F715}' => Some("F18".to_string()),
        '\u{F716}' => Some("F19".to_string()),
        '\u{F717}' => Some("F20".to_string()),
        '\u{F718}' => Some("F21".to_string()),
        '\u{F719}' => Some("F22".to_string()),
        '\u{F71A}' => Some("F23".to_string()),
        '\u{F71B}' => Some("F24".to_string()),
        '\u{F727}' => Some("Insert".to_string()),
        '\u{F729}' => Some("Home".to_string()),
        '\u{F72B}' => Some("End".to_string()),
        '\u{F72C}' => Some("PageUp".to_string()),
        '\u{F72D}' => Some("PageDown".to_string()),
        character if character.is_ascii_alphanumeric() => {
            Some(character.to_ascii_uppercase().to_string())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        api_profile_options_for_display, captured_hotkey_label, captured_key_label,
        language_options_for_display, log_level_options_for_display, model_label,
        model_options_for_display, recording_mode_options_for_display,
    };
    use crate::settings::{AppLanguage, RecordingMode};

    #[test]
    fn empty_model_label_uses_default_transcription_model() {
        assert_eq!(model_label(""), "openai/whisper-large-v3-turbo");
        assert_eq!(model_label(" custom "), "custom");
    }

    #[test]
    fn model_options_include_current_model_first_and_deduplicate() {
        let options = vec![
            "openai/whisper-large-v3-turbo".to_string(),
            "custom".to_string(),
            " custom ".to_string(),
        ];

        assert_eq!(
            model_options_for_display("custom", &options),
            vec![
                "custom".to_string(),
                "openai/whisper-large-v3-turbo".to_string()
            ]
        );
    }

    #[test]
    fn api_profile_options_include_active_profile_first() {
        let mut settings = crate::settings::AppSettings::default().normalized();
        settings.active_api_profile_id = "groq".to_string();

        assert_eq!(
            api_profile_options_for_display(&settings),
            vec!["groq".to_string(), "ai2npu".to_string()]
        );
    }

    #[test]
    fn recording_mode_options_include_current_mode_first() {
        assert_eq!(
            recording_mode_options_for_display(RecordingMode::Hold, AppLanguage::Russian)[0],
            "Удержание"
        );
        assert_eq!(
            recording_mode_options_for_display(RecordingMode::Hold, AppLanguage::English)[0],
            "Hold"
        );
    }

    #[test]
    fn log_level_options_include_current_level_first() {
        assert_eq!(
            log_level_options_for_display("Debug", AppLanguage::Russian)[0],
            "Отладка"
        );
        assert_eq!(
            log_level_options_for_display("Debug", AppLanguage::English)[0],
            "Debug"
        );
    }

    #[test]
    fn language_options_include_current_language_first() {
        assert_eq!(
            language_options_for_display(AppLanguage::English)[0],
            "English"
        );
        assert_eq!(
            language_options_for_display(AppLanguage::Russian),
            vec!["Русский".to_string(), "English".to_string()]
        );
    }

    #[test]
    fn captured_hotkey_label_normalizes_shortcut() {
        assert_eq!(
            captured_hotkey_label(" ", true, false, false, false).unwrap(),
            Some("Ctrl+Space".to_string())
        );
        assert_eq!(
            captured_hotkey_label("y", false, true, false, false).unwrap(),
            Some("Alt+Y".to_string())
        );
    }

    #[test]
    fn captured_hotkey_label_ignores_modifier_only_press() {
        assert_eq!(
            captured_hotkey_label("\u{0011}", true, false, false, false).unwrap(),
            None
        );
    }

    #[test]
    fn captured_hotkey_label_accepts_single_key() {
        assert_eq!(
            captured_hotkey_label("Y", false, false, false, false).unwrap(),
            Some("Y".to_string())
        );
        assert_eq!(
            captured_hotkey_label("F23", false, false, false, false).unwrap(),
            Some("F23".to_string())
        );
    }

    #[test]
    fn captured_hotkey_label_accepts_copilot_chord() {
        assert_eq!(
            captured_hotkey_label("F23", false, false, true, true).unwrap(),
            Some("Shift+Win+F23".to_string())
        );
        assert_eq!(
            captured_hotkey_label("\u{F71A}", false, false, true, true).unwrap(),
            Some("Shift+Win+F23".to_string())
        );
    }

    #[test]
    fn captured_key_label_maps_named_keys() {
        assert_eq!(captured_key_label("\u{F72C}"), Some("PageUp".to_string()));
        assert_eq!(captured_key_label("\u{001b}"), Some("Esc".to_string()));
        assert_eq!(captured_key_label("\u{F71A}"), Some("F23".to_string()));
        assert_eq!(captured_key_label("F23"), Some("F23".to_string()));
    }
}
