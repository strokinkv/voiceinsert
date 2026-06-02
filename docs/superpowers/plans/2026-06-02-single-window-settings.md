# Single-Window Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the tabbed/sidebar settings window with a single-screen `1360x800` masonry settings panel and default audio API language to `ru`.

**Architecture:** Keep behavior changes small and test-first: first make audio requests default `language=ru`, then remove the editable profile language field from the Rust/Slint UI contract, then rebuild the settings Slint layout as one static masonry panel. Reuse existing autosave, profile management, model loading, log-folder opening, and settings persistence flows.

**Tech Stack:** Rust 2024, Slint 1.14, Tokio, reqwest multipart, wiremock tests.

---

## File Structure

- Modify `src/api/transcription.rs`: add a default input language constant and send `language=ru` for transcription and translation when no explicit language is provided.
- Modify `tests/e2e_api_tests.rs`: assert multipart requests include `language=ru` for both audio endpoints.
- Modify `src/ui.rs`: remove `SettingsEdit.language`, stop setting/reading the Slint `language` property, keep profile metadata behavior.
- Modify `src/app/commands.rs`: stop copying UI language edits into `ApiProfile.language`; existing stored values remain in settings files for compatibility.
- Modify `src/app/mod.rs`: pass `None` for `AudioTask.language` in normal runtime requests and let `send_audio` apply the default.
- Modify `ui/components.slint`: add vertical field/dropdown helpers, compact card helpers, hint marker, and centered toast presentation; keep existing components if still used elsewhere.
- Modify `ui/settings.slint`: remove sidebar/page switching and render the one-screen masonry layout.
- Modify `README.md` and `README_ru.md`: update the settings-window behavior and mention default Russian input language.

---

### Task 1: Default Audio Request Language

**Files:**
- Modify: `src/api/transcription.rs`
- Modify: `tests/e2e_api_tests.rs`

- [ ] **Step 1: Write failing multipart tests for default `language=ru`**

Replace `tests/e2e_api_tests.rs` with:

```rust
use voiceinsert::api::transcription::{AudioRequestKind, SendAudioRequest, send_audio};
use voiceinsert::audio::recorder::encode_wav_mono_16khz_i16;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn transcription_defaults_language_to_russian() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(body_string_contains("language"))
        .and(body_string_contains("ru"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello from mock"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "openai/whisper-large-v3-turbo",
            language: None,
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "hello from mock");
}

#[tokio::test]
async fn translation_defaults_input_language_to_russian() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/translations"))
        .and(body_string_contains("language"))
        .and(body_string_contains("ru"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "translated text"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "whisper-large-v3",
            language: None,
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Translation,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "translated text");
}

#[tokio::test]
async fn explicit_language_overrides_default_language() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .and(body_string_contains("language"))
        .and(body_string_contains("en"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "english text"
        })))
        .mount(&server)
        .await;

    let text = send_audio(
        &reqwest::Client::new(),
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "openai/whisper-large-v3-turbo",
            language: Some("en"),
            temperature: Some(0.2),
            wav_bytes: encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap(),
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "english text");
}
```

- [ ] **Step 2: Run the E2E API tests and verify failure**

Run: `cargo test --test e2e_api_tests`

Expected: the first two tests fail because `send_audio` currently omits `language` when `language: None`, and translation never sends a language field.

- [ ] **Step 3: Implement default audio input language**

In `src/api/transcription.rs`, add the constant near `AUDIO_RESPONSE_FORMAT`:

```rust
pub const DEFAULT_INPUT_LANGUAGE: &str = "ru";
```

Replace the language form block in `send_audio` with:

```rust
    let language = audio
        .language
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .unwrap_or(DEFAULT_INPUT_LANGUAGE);
    form = form.text("language", language.to_string());
```

This block must run for both `AudioRequestKind::Transcription` and `AudioRequestKind::Translation`.

- [ ] **Step 4: Run the E2E API tests and verify pass**

Run: `cargo test --test e2e_api_tests`

Expected: all E2E API tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/api/transcription.rs tests/e2e_api_tests.rs
git commit -m "fix: default audio input language to russian"
```

---

### Task 2: Remove Language From Settings UI Contract

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/app/commands.rs`
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Write a failing unit test that UI edits preserve stored profile language**

In `src/app/mod.rs`, update the `apply_settings_edit_updates_active_profile_and_hotkeys` test's `SettingsEdit` literal by removing the `language` field. Before calling `apply_settings_edit`, set the active profile language:

```rust
        let mut settings = crate::settings::AppSettings::default().normalized();
        settings.api_profiles[0].language = "en".to_string();
```

Keep the rest of the test, but change the language assertion to:

```rust
        assert_eq!(settings.active_profile().language, "en");
```

Expected final shape at the start of the test:

```rust
    fn apply_settings_edit_updates_active_profile_and_hotkeys() {
        let mut settings = crate::settings::AppSettings::default().normalized();
        settings.api_profiles[0].language = "en".to_string();
        let settings = apply_settings_edit(
            settings,
            SettingsEdit {
                profile_name: "ai2npu".to_string(),
                base_url: " http://localhost:9555 ".to_string(),
                api_key: String::new(),
                model: " whisper-large-v3 ".to_string(),
                temperature: "0.4".to_string(),
                request_timeout_seconds: "45".to_string(),
                transcription_hotkey: " Ctrl+Space ".to_string(),
                translation_hotkey: " Alt+Y ".to_string(),
                recording_mode: "Hold".to_string(),
                silence_threshold_percent: "8".to_string(),
                silence_timeout_milliseconds: "900".to_string(),
                max_recording_seconds: "30".to_string(),
                start_with_windows: true,
                ui_language: "English".to_string(),
                enable_sounds: false,
                restore_clipboard_content: false,
                delay_before_paste_milliseconds: "120".to_string(),
                delay_before_clipboard_restore_milliseconds: "500".to_string(),
                log_level: "Debug".to_string(),
            },
        )
        .normalized();
```

- [ ] **Step 2: Run the targeted test and verify failure**

Run: `cargo test apply_settings_edit_updates_active_profile_and_hotkeys -- --exact`

Expected: compile failure because `SettingsEdit` still requires `language`.

- [ ] **Step 3: Remove `language` from `SettingsEdit` and profile editing**

In `src/ui.rs`, delete this field from `SettingsEdit`:

```rust
    pub language: String,
```

In `UiController::open_settings`, remove:

```rust
        window.set_language(SharedString::from(profile.language.as_str()));
```

In `UiController::settings_edit`, remove:

```rust
            language: window.get_language().to_string(),
```

In `src/app/commands.rs`, remove this assignment from `apply_settings_edit`:

```rust
        profile.language = edit.language.trim().to_string();
```

In `src/app/mod.rs`, change `stop_recording_and_insert` to let `send_audio` apply the default:

```rust
                language: None,
```

- [ ] **Step 4: Accept localized dropdown values in parsers**

In `src/app/commands.rs`, replace `parse_recording_mode`, `parse_language`, and `parse_log_level` with:

```rust
fn parse_recording_mode(value: &str) -> RecordingMode {
    match value.trim() {
        "Hold" | "Удержание" => RecordingMode::Hold,
        "SilenceTimeout" | "Тишина" => RecordingMode::SilenceTimeout,
        _ => RecordingMode::Toggle,
    }
}

fn parse_language(value: &str) -> AppLanguage {
    match value.trim() {
        "English" => AppLanguage::English,
        _ => AppLanguage::Russian,
    }
}

fn parse_log_level(value: &str) -> &'static str {
    match value.trim() {
        "Debug" | "Отладка" => "Debug",
        "Warning" | "Предупреждения" => "Warning",
        "Error" | "Ошибки" => "Error",
        _ => "Information",
    }
}
```

- [ ] **Step 5: Run targeted tests**

Run:

```powershell
cargo test apply_settings_edit_updates_active_profile_and_hotkeys -- --exact
cargo test --test settings_tests
```

Expected: both commands pass.

- [ ] **Step 6: Commit**

```powershell
git add src/ui.rs src/app/commands.rs src/app/mod.rs
git commit -m "refactor: remove language field from settings ui"
```

---

### Task 3: Add Single-Window Slint Components

**Files:**
- Modify: `ui/components.slint`
- Modify: `build.rs`

- [ ] **Step 1: Add build tracking for shared Slint components**

In `build.rs`, add:

```rust
    println!("cargo:rerun-if-changed=ui/components.slint");
```

directly after the existing `ui/settings.slint` line.

- [ ] **Step 2: Add new reusable components**

Append these components to `ui/components.slint`:

```slint
export component HintLabel inherits HorizontalLayout {
    in property <string> label: "";
    in property <string> hint: "";

    height: 18px;
    spacing: 6px;

    Label {
        label: root.label;
        vertical-alignment: center;
    }

    Rectangle {
        visible: root.hint != "";
        width: 16px;
        height: 16px;
        background: hint-touch.has-hover ? #dbe7f5 : #eef3f8;
        border-color: #b8c7d8;
        border-width: 1px;
        border-radius: 8px;

        Text {
            text: "?";
            color: #516579;
            font-size: 10px;
            font-weight: 700;
            horizontal-alignment: center;
            vertical-alignment: center;
        }

        hint-touch := TouchArea { width: parent.width; height: parent.height; }
    }
}

export component VerticalField inherits VerticalLayout {
    in property <string> label: "";
    in property <string> hint: "";
    in-out property <string> value: "";
    callback edited();

    spacing: 5px;
    height: 57px;

    HintLabel { label: root.label; hint: root.hint; }
    Field {
        value <=> root.value;
        horizontal-stretch: 1;
        edited => { root.edited(); }
    }
}

export component VerticalCombo inherits VerticalLayout {
    in property <string> label: "";
    in property <string> hint: "";
    in-out property <string> value: "";
    in property <[string]> options: [];
    callback selected(value: string);

    spacing: 5px;
    height: 57px;

    HintLabel { label: root.label; hint: root.hint; }
    ComboBox {
        model: root.options;
        current-value <=> root.value;
        horizontal-stretch: 1;
        selected(value) => {
            root.value = value;
            root.selected(value);
        }
    }
}

export component SettingsCard inherits Rectangle {
    background: #ffffff;
    border-color: #d8e0ea;
    border-width: 1px;
    border-radius: 8px;
}

export component Toast inherits Rectangle {
    in property <string> message: "";

    visible: root.message != "";
    width: 420px;
    height: 54px;
    background: #fff6f2;
    border-color: #e2ad9b;
    border-width: 1px;
    border-radius: 8px;

    Text {
        text: root.message;
        x: 14px;
        y: 8px;
        width: parent.width - 28px;
        height: parent.height - 16px;
        color: #77402f;
        font-size: 12px;
        wrap: word-wrap;
        vertical-alignment: center;
    }
}
```

- [ ] **Step 3: Run Slint compile check**

Run: `cargo check`

Expected: this may fail if `ComboBox` is not in scope for appended components. If it fails with an unknown `ComboBox`, keep the existing `import { ComboBox } from "std-widgets.slint";` at the top of `ui/components.slint`; do not duplicate it.

- [ ] **Step 4: Commit**

```powershell
git add build.rs ui/components.slint
git commit -m "feat: add settings panel slint components"
```

---

### Task 4: Replace Sidebar Settings Layout With Masonry Panel

**Files:**
- Modify: `ui/settings.slint`
- Modify: `src/ui.rs`

- [ ] **Step 1: Localize current dropdown values before they reach Slint**

In `src/ui.rs`, replace the `window.set_recording_mode(...)` call in `UiController::open_settings` with:

```rust
        window.set_recording_mode(SharedString::from(recording_mode_display_label(
            settings.recording_mode,
            settings.ui_language,
        )));
```

Replace the `window.set_log_level(...)` call with:

```rust
        window.set_log_level(SharedString::from(log_level_display_label(
            &settings.log_level,
            settings.ui_language,
        )));
```

Add these helpers near the existing label helpers:

```rust
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
```

Leave the old `recording_mode_label` helper in place only if another call still uses it. Delete it if Rust reports it as unused.

- [ ] **Step 2: Remove page navigation imports and properties**

At the top of `ui/settings.slint`, change the import to:

```slint
import {
    ComboRow, FieldRow, Label, Section, SmallButton, TinyButton, ToggleRow, ValueText,
    SettingsCard, Toast, VerticalCombo, VerticalField
} from "./components.slint";
```

Remove these properties:

```slint
    in-out property <string> profile-line: "";
    in-out property <string> language: "";
    in-out property <string> logs-folder: "";
    in-out property <int> selected-page: 0;
    property <length> content-width: root.width - 290px;
```

Keep `status-text` for toast input.

- [ ] **Step 3: Set the new window shell**

In `SettingsWindow`, set:

```slint
    width: 1360px;
    height: 800px;
    background: #eef3f8;
```

- [ ] **Step 4: Replace the whole root layout**

Replace the current `HorizontalLayout { ... }` body with this fixed masonry layout:

```slint
    Rectangle {
        background: #eef3f8;
        width: parent.width;
        height: parent.height;

        SettingsCard {
            x: 16px;
            y: 16px;
            width: 540px;
            height: 452px;

            VerticalLayout {
                padding: 20px;
                spacing: 12px;

                HorizontalLayout {
                    height: 57px;
                    spacing: 8px;

                    VerticalCombo {
                        label: root.is-russian ? "Активный профиль" : "Active profile";
                        value <=> root.active-profile-name;
                        options: root.api-profile-options;
                        horizontal-stretch: 1;
                        selected(value) => { root.select-api-profile(value); }
                    }

                    TinyButton { label: "+"; clicked => { root.add-api-profile(); } }
                    TinyButton { label: "-"; clicked => { root.delete-api-profile(); } }
                }

                VerticalField { label: root.is-russian ? "Имя профиля" : "Profile name"; value <=> root.profile-name; edited => { root.settings-changed(); } }
                VerticalField { label: "Base URL"; value <=> root.api-base-url; edited => { root.settings-changed(); } }
                VerticalField { label: "API key"; value <=> root.api-key; edited => { root.settings-changed(); } }
                VerticalCombo { label: root.is-russian ? "Модель" : "Model"; value <=> root.model-name; options: root.model-options; selected(value) => { root.settings-changed(); } }

                HorizontalLayout {
                    height: 57px;
                    spacing: 12px;
                    VerticalField { label: root.is-russian ? "Температура" : "Temperature"; hint: root.is-russian ? "0.0-1.0, ниже стабильнее" : "0.0-1.0, lower is steadier"; value <=> root.temperature; horizontal-stretch: 1; edited => { root.settings-changed(); } }
                    VerticalField { label: root.is-russian ? "Таймаут, сек" : "Timeout, sec"; hint: root.is-russian ? "Время ожидания API" : "API request timeout"; value <=> root.request-timeout-seconds; horizontal-stretch: 1; edited => { root.settings-changed(); } }
                }
            }
        }

        SettingsCard {
            x: 572px;
            y: 16px;
            width: 442px;
            height: 452px;

            VerticalLayout {
                padding: 20px;
                spacing: 12px;

                VerticalField { label: root.is-russian ? "Клавиша транскрибации" : "Transcription hotkey"; value <=> root.transcription-hotkey; edited => { root.settings-changed(); } }
                VerticalField { label: root.is-russian ? "Клавиша перевода" : "Translation hotkey"; value <=> root.translation-hotkey; edited => { root.settings-changed(); } }
                VerticalCombo {
                    label: root.is-russian ? "Режим записи" : "Recording mode";
                    value <=> root.recording-mode;
                    options: [root.is-russian ? "Переключатель" : "Toggle", root.is-russian ? "Удержание" : "Hold", root.is-russian ? "Тишина" : "SilenceTimeout"];
                    selected(value) => {
                        root.recording-mode = value;
                        root.settings-changed();
                    }
                }
                VerticalField { label: root.is-russian ? "Порог тишины, %" : "Silence threshold, %"; hint: root.is-russian ? "Уровень, ниже которого звук считается тишиной" : "Level treated as silence"; value <=> root.silence-threshold-percent; edited => { root.settings-changed(); } }
                VerticalField { label: root.is-russian ? "Таймаут тишины, мс" : "Silence timeout, ms"; hint: root.is-russian ? "Сколько ждать тишину перед остановкой" : "Silence duration before stop"; value <=> root.silence-timeout-milliseconds; edited => { root.settings-changed(); } }
                VerticalField { label: root.is-russian ? "Макс. длительность, сек" : "Max duration, sec"; hint: root.is-russian ? "Жесткий лимит одной записи" : "Hard limit for one recording"; value <=> root.max-recording-seconds; edited => { root.settings-changed(); } }
            }
        }

        SettingsCard {
            x: 1030px;
            y: 16px;
            width: 344px;
            height: 218px;

            VerticalLayout {
                padding: 20px;
                spacing: 12px;
                ToggleRow { label: root.is-russian ? "Запускать вместе с Windows" : "Start with Windows"; checked <=> root.start-with-windows; toggled => { root.settings-changed(); } }
                ToggleRow { label: root.is-russian ? "Звуковое оповещение" : "Sound alerts"; checked <=> root.enable-sounds; toggled => { root.settings-changed(); } }
                VerticalCombo {
                    label: root.is-russian ? "Язык интерфейса" : "Interface language";
                    value <=> root.ui-language;
                    options: ["Russian", "English"];
                    selected(value) => { root.ui-language = value; root.settings-changed(); }
                }
            }
        }

        SettingsCard {
            x: 1030px;
            y: 250px;
            width: 344px;
            height: 218px;

            VerticalLayout {
                padding: 20px;
                spacing: 14px;
                SmallButton { label: root.is-russian ? "Открыть папку логов" : "Open logs folder"; width: 180px; clicked => { root.open-logs-folder(); } }
                VerticalCombo {
                    label: root.is-russian ? "Уровень логов" : "Log level";
                    hint: root.is-russian ? "Детальность диагностических сообщений" : "Diagnostic message detail";
                    value <=> root.log-level;
                    options: [root.is-russian ? "Инфо" : "Information", root.is-russian ? "Отладка" : "Debug", root.is-russian ? "Предупреждения" : "Warning", root.is-russian ? "Ошибки" : "Error"];
                    selected(value) => { root.log-level = value; root.settings-changed(); }
                }
            }
        }

        SettingsCard {
            x: 16px;
            y: 484px;
            width: 540px;
            height: 230px;

            VerticalLayout {
                padding: 20px;
                spacing: 12px;
                ToggleRow { label: root.is-russian ? "Восстанавливать буфер" : "Restore clipboard"; checked <=> root.restore-clipboard-content; toggled => { root.settings-changed(); } }
                VerticalField { label: root.is-russian ? "Задержка перед вставкой, мс" : "Delay before paste, ms"; hint: root.is-russian ? "Пауза перед отправкой вставки" : "Pause before paste"; value <=> root.delay-before-paste-milliseconds; edited => { root.settings-changed(); } }
                VerticalField { label: root.is-russian ? "Задержка перед восстановлением, мс" : "Delay before restore, ms"; hint: root.is-russian ? "Пауза перед возвратом прежнего буфера" : "Pause before restoring clipboard"; value <=> root.delay-before-clipboard-restore-milliseconds; edited => { root.settings-changed(); } }
            }
        }

        Toast {
            x: (parent.width - self.width) / 2;
            y: (parent.height - self.height) / 2;
            message: root.status-text;
        }
    }
```

- [ ] **Step 5: Run compile and apply the listed Slint syntax corrections**

Run: `cargo check`

Expected: Slint may require corrections around inline array expressions. If a localized inline array fails, add explicit properties to `SettingsWindow`:

```slint
    property <[string]> recording-mode-options: root.is-russian ? ["Переключатель", "Удержание", "Тишина"] : ["Toggle", "Hold", "SilenceTimeout"];
    property <[string]> log-level-options: root.is-russian ? ["Инфо", "Отладка", "Предупреждения", "Ошибки"] : ["Information", "Debug", "Warning", "Error"];
```

Then use:

```slint
                    options: root.recording-mode-options;
```

and:

```slint
                    options: root.log-level-options;
```

If `self.width` fails in `Toast`, replace the centered bindings with:

```slint
            x: (parent.width - 420px) / 2;
            y: (parent.height - 54px) / 2;
```

- [ ] **Step 6: Commit**

```powershell
git add ui/settings.slint src/ui.rs
git commit -m "feat: redesign settings as single panel"
```

---

### Task 5: Align Status Handling With Toast Errors

**Files:**
- Modify: `src/app/commands.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/ui.rs`

- [ ] **Step 1: Stop showing routine success statuses**

In `src/app/commands.rs`, remove these success status calls:

```rust
                    self.ui
                        .set_status(settings_saved_status(self.settings.ui_language));
```

```rust
                self.ui
                    .set_status(api_profile_added_status(self.settings.ui_language));
```

```rust
                    self.ui
                        .set_status(api_profile_deleted_status(self.settings.ui_language));
```

```rust
                    self.ui
                        .set_status(api_profile_selected_status(self.settings.ui_language));
```

Keep error/problem status calls, including autosave errors and `api_profile_required_status`.

- [ ] **Step 2: Stop showing model-loading success statuses**

In `src/app/mod.rs`, remove:

```rust
                        self.ui
                            .set_status(models_loaded_status(self.settings.ui_language));
```

and:

```rust
        self.ui
            .set_status(models_loading_status(self.settings.ui_language));
```

Keep `BackgroundEvent::Error(message) => self.ui.set_status(message)`.

- [ ] **Step 3: Clear toast when opening settings**

In `UiController::open_settings`, keep:

```rust
        window.set_status_text(SharedString::from(""));
```

This ensures old errors do not reappear when reopening the window.

- [ ] **Step 4: Run compile check**

Run: `cargo check`

Expected: pass, though helper functions like `settings_saved_status`, `models_loading_status`, and `models_loaded_status` may now be unused. Remove unused imports/functions only if Rust reports warnings promoted to errors.

- [ ] **Step 5: Commit**

```powershell
git add src/app/commands.rs src/app/mod.rs src/ui.rs
git commit -m "refactor: show settings messages only as error toasts"
```

---

### Task 6: Documentation Updates

**Files:**
- Modify: `README.md`
- Modify: `README_ru.md`

- [ ] **Step 1: Update English settings description**

In `README.md`, replace:

```text
- Settings use sidebar navigation, compact profile controls, one model field, and a global status bar.
```

with:

```text
- Settings use a single-screen card layout without tabs or scrolling. API language input is not shown; audio requests default to Russian (`ru`) as the input language.
```

- [ ] **Step 2: Update Russian settings description**

In `README_ru.md`, replace:

```text
- Окно настроек использует sidebar-навигацию, компактное управление профилями, одно поле модели и общую строку состояния.
```

with:

```text
- Окно настроек использует одноэкранную карточную компоновку без вкладок и прокрутки. Язык ввода API не показывается; аудиозапросы по умолчанию используют русский (`ru`) как язык ввода.
```

- [ ] **Step 3: Run formatting and tests**

Run:

```powershell
cargo fmt --check
cargo test
```

Expected: both pass.

- [ ] **Step 4: Commit**

```powershell
git add README.md README_ru.md
git commit -m "docs: update settings window description"
```

---

### Task 7: Final Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run full Rust verification**

Run:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

Expected: all commands pass.

- [ ] **Step 2: Launch the app for visual inspection**

Run:

```powershell
cargo run --bin VoiceInsert
```

Open settings from the tray.

Expected visual checks:

- Window is `1360x800`.
- No sidebar, tabs, title header, app name, active-profile header, bottom status bar, or scrollbars.
- Card outer spacing is tight, roughly `14px` to `18px` from the window edges.
- Cards have no group headings or icons.
- All agreed fields are visible.
- `Language / Язык распознавания` is absent.
- All former option-button groups are dropdowns.
- Toggle controls remain toggles.
- Log folder path is not displayed.
- `Открыть папку логов` works.
- Error toast overlays the center of the settings panel and does not shift cards.

- [ ] **Step 3: Commit any final fixes**

If visual verification requires layout corrections, commit them:

```powershell
git add ui/settings.slint ui/components.slint src
git commit -m "fix: polish single-window settings layout"
```

Expected: no commit is needed if Step 2 passes.
