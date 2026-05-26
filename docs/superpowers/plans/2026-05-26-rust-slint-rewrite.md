# VoiceInsert Rust + Slint Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the C# WPF VoiceInsert application with a Rust-only Windows 11 tray app using Slint while preserving the technical specification.

**Architecture:** The Rust app is a Cargo workspace with one Windows desktop binary. Core behavior lives in testable Rust modules independent of Slint; Slint owns the settings and overlay UI only. Windows integrations use focused adapters around crates or the `windows` crate.

**Tech Stack:** Rust 1.95, Slint, Tokio, Reqwest, Serde, Tracing, CPAL, Hound, Windows crate, Inno Setup, GitHub Actions on Windows.

---

## File Map

- Create `Cargo.toml`: workspace/package metadata, dependencies, binary name `VoiceInsert`.
- Create `build.rs`: Slint compilation and Windows resource embedding if needed.
- Create `src/main.rs`: process startup and application bootstrap.
- Create `src/app.rs`: lifecycle orchestration, recording state machine, wiring.
- Create `src/settings.rs`: app settings, profile defaults, migration, validation.
- Create `src/i18n.rs`: Russian and English text constants.
- Create `src/logging.rs`: tracing setup and last-error state.
- Create `src/secrets.rs`: DPAPI encrypted API key storage.
- Create `src/api/endpoints.rs`: base URL validation and endpoint construction.
- Create `src/api/models.rs`: model loading client.
- Create `src/api/transcription.rs`: transcription and translation client.
- Create `src/audio/*`: device listing, capture, levels, WAV encoding.
- Create `src/hotkeys/*`: hotkey parsing and global registration.
- Create `src/clipboard.rs`: clipboard paste and restore.
- Create `src/tray.rs`: tray icon, menu, localized labels, state icons.
- Create `src/overlay.rs`: Slint overlay adapter and waveform state.
- Create `ui/settings.slint`: settings window.
- Create `ui/overlay.slint`: recording overlay.
- Move/reuse `src/VoiceInsert.App/Assets/*` into `assets/`.
- Modify `installer/VoiceInsert.iss`: package Rust release output.
- Modify `scripts/*.ps1`: Cargo build/test/package flow.
- Modify `.github/workflows/*.yml`: Rust CI/release flow.
- Modify `README.md`, `README_ru.md`, `CHANGELOG.md`, docs release notes as needed.
- Delete `VoiceInsert.slnx`, `Directory.Build.props`, `NuGet.Config`, `src/VoiceInsert.App`, `tests/VoiceInsert.App.Tests`, `TestResults`, `.dotnet`, `.nuget` after Rust replacement is functional.

## Execution Notes

Work is on branch `rewrite-rust-slint`.

Subagent execution should not run multiple implementers against the same files. Safe parallel research/review areas are:

- settings/API/secrets;
- audio/hotkeys/clipboard/tray;
- Slint UI/overlay;
- installer/scripts/docs.

Implementation commits should remain task-sized. Do not remove the C# project until the Rust app builds and core tests pass.

---

### Task 1: Rust Workspace Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `build.rs`
- Create: `src/main.rs`
- Create: `src/app.rs`
- Create: `src/lib.rs`
- Create: `assets/VoiceInsert.ico`
- Create: `assets/record-start.wav`
- Create: `assets/record-stop.wav`

- [ ] **Step 1: Create the Cargo package**

Add `Cargo.toml` with this starting shape:

```toml
[package]
name = "voiceinsert"
version = "1.1.0"
edition = "2024"
build = "build.rs"

[[bin]]
name = "VoiceInsert"
path = "src/main.rs"

[dependencies]
anyhow = "1"
arboard = "3"
cpal = "0.16"
global-hotkey = "0.7"
hound = "3"
reqwest = { version = "0.12", features = ["json", "multipart", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
slint = "1.14"
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
tracing = "0.1"
tracing-appender = "0.2"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tray-icon = "0.21"
windows = { version = "0.62", features = [
  "Win32_Foundation",
  "Win32_Graphics_Gdi",
  "Win32_Security_Cryptography",
  "Win32_System_DataExchange",
  "Win32_System_LibraryLoader",
  "Win32_UI_Input_KeyboardAndMouse",
  "Win32_UI_Shell",
  "Win32_UI_WindowsAndMessaging"
] }

[build-dependencies]
slint-build = "1.14"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Add minimal source modules**

Create `src/lib.rs`:

```rust
pub mod app;
pub mod settings;
```

Create `src/main.rs`:

```rust
fn main() -> anyhow::Result<()> {
    voiceinsert::app::run()
}
```

Create `src/app.rs`:

```rust
pub fn run() -> anyhow::Result<()> {
    Ok(())
}
```

- [ ] **Step 3: Add build script**

Create `build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=ui/overlay.slint");
}
```

- [ ] **Step 4: Move assets without changing bytes**

Copy the current assets from `src/VoiceInsert.App/Assets/` into `assets/`.

Run:

```powershell
cargo build
```

Expected: build succeeds and produces `target\debug\VoiceInsert.exe`.

- [ ] **Step 5: Commit**

```powershell
git add Cargo.toml build.rs src assets
git commit -m "chore: add rust workspace skeleton"
```

---

### Task 2: Settings, Profiles, Localization, and Tests

**Files:**
- Create: `src/settings.rs`
- Create: `src/i18n.rs`
- Modify: `src/lib.rs`
- Create: `tests/settings_tests.rs`

- [ ] **Step 1: Write failing settings tests**

Create `tests/settings_tests.rs`:

```rust
use voiceinsert::settings::{AppLanguage, AppSettings, RecordingMode};

#[test]
fn defaults_create_ai2npu_and_groq_profiles() {
    let settings = AppSettings::default().normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(settings.api_profiles[1].name, "groq");
    assert_eq!(settings.api_profiles[1].base_url, "https://api.groq.com/openai/");
    assert_eq!(settings.api_profiles[1].model, "whisper-large-v3");
}

#[test]
fn default_hotkeys_and_language_match_spec() {
    let settings = AppSettings::default().normalized();

    assert_eq!(settings.hotkey, "Ctrl+Space");
    assert_eq!(settings.translation_hotkey, "Alt+Y");
    assert_eq!(settings.ui_language, AppLanguage::Russian);
    assert_eq!(settings.recording_mode, RecordingMode::Toggle);
}

#[test]
fn legacy_default_profile_migrates_to_ai2npu() {
    let mut settings = AppSettings::default();
    settings.api_profiles.clear();
    settings.api_profiles.push(voiceinsert::settings::ApiProfile {
        id: "legacy".to_string(),
        name: "Default".to_string(),
        base_url: "http://127.0.0.1:9573".to_string(),
        model: "legacy-model".to_string(),
        language: "ru".to_string(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    });
    settings.active_api_profile_id = "legacy".to_string();

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(settings.active_profile().model, "legacy-model");
}

#[test]
fn legacy_wlast_profile_migrates_to_ai2npu() {
    let mut settings = AppSettings::default();
    settings.api_profiles.clear();
    settings.api_profiles.push(voiceinsert::settings::ApiProfile {
        id: "legacy-wlast".to_string(),
        name: "wlast".to_string(),
        base_url: "http://127.0.0.1:9573".to_string(),
        model: "legacy-model".to_string(),
        language: "ru".to_string(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    });
    settings.active_api_profile_id = "legacy-wlast".to_string();

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
}

#[test]
fn numeric_settings_are_clamped() {
    let mut settings = AppSettings::default();
    settings.temperature = 4.2;
    settings.request_timeout_seconds = 1;
    settings.silence_timeout_milliseconds = 50;
    settings.max_recording_seconds = 0;

    let settings = settings.normalized();

    assert_eq!(settings.temperature, 1.0);
    assert_eq!(settings.request_timeout_seconds, 5);
    assert_eq!(settings.silence_timeout_milliseconds, 100);
    assert_eq!(settings.max_recording_seconds, 1);
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --test settings_tests
```

Expected: fails because `settings` module is not implemented.

- [ ] **Step 3: Implement settings models**

Create `src/settings.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingMode {
    Toggle,
    Hold,
    SilenceTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLanguage {
    Russian,
    English,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub language: String,
    pub temperature: f64,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub settings_version: u32,
    pub active_api_profile_id: String,
    pub api_profiles: Vec<ApiProfile>,
    pub hotkey: String,
    pub translation_hotkey: String,
    pub recording_mode: RecordingMode,
    pub silence_threshold_percent: u8,
    pub silence_timeout_milliseconds: u64,
    pub max_recording_seconds: u64,
    pub start_with_windows: bool,
    pub ui_language: AppLanguage,
    pub launch_minimized_to_tray: bool,
    pub show_floating_recording_window: bool,
    pub enable_sounds: bool,
    pub restore_clipboard_content: bool,
    pub delay_before_paste_milliseconds: u64,
    pub delay_before_clipboard_restore_milliseconds: u64,
    pub log_level: String,
    pub temperature: f64,
    pub request_timeout_seconds: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            settings_version: 8,
            active_api_profile_id: String::new(),
            api_profiles: Vec::new(),
            hotkey: "Ctrl+Space".to_string(),
            translation_hotkey: "Alt+Y".to_string(),
            recording_mode: RecordingMode::Toggle,
            silence_threshold_percent: 4,
            silence_timeout_milliseconds: 1200,
            max_recording_seconds: 120,
            start_with_windows: false,
            ui_language: AppLanguage::Russian,
            launch_minimized_to_tray: true,
            show_floating_recording_window: true,
            enable_sounds: true,
            restore_clipboard_content: true,
            delay_before_paste_milliseconds: 80,
            delay_before_clipboard_restore_milliseconds: 300,
            log_level: "Information".to_string(),
            temperature: 0.2,
            request_timeout_seconds: 120,
        }
    }
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.settings_version = 8;
        self.hotkey = normalize_hotkey_or(&self.hotkey, "Ctrl+Space");
        self.translation_hotkey = normalize_hotkey_or(&self.translation_hotkey, "Alt+Y");
        if self.hotkey.eq_ignore_ascii_case(&self.translation_hotkey) {
            self.translation_hotkey = if self.hotkey.eq_ignore_ascii_case("Alt+Y") {
                "Ctrl+Alt+Y".to_string()
            } else {
                "Alt+Y".to_string()
            };
        }
        self.temperature = clamp_f64((self.temperature * 10.0).round() / 10.0, 0.0, 1.0);
        self.request_timeout_seconds = self.request_timeout_seconds.clamp(5, 600);
        self.silence_threshold_percent = self.silence_threshold_percent.min(100);
        self.silence_timeout_milliseconds = self.silence_timeout_milliseconds.clamp(100, 30_000);
        self.max_recording_seconds = self.max_recording_seconds.clamp(1, 3600);
        self.delay_before_paste_milliseconds = self.delay_before_paste_milliseconds.min(5000);
        self.delay_before_clipboard_restore_milliseconds =
            self.delay_before_clipboard_restore_milliseconds.min(30_000);
        self.ensure_profiles();
        self
    }

    pub fn active_profile(&self) -> &ApiProfile {
        self.api_profiles
            .iter()
            .find(|profile| profile.id == self.active_api_profile_id)
            .unwrap_or(&self.api_profiles[0])
    }

    fn ensure_profiles(&mut self) {
        if self.api_profiles.is_empty() {
            self.api_profiles.push(ai2npu_profile());
        }

        for profile in &mut self.api_profiles {
            let is_legacy_local_profile = profile.name.eq_ignore_ascii_case("Default")
                || profile.name.eq_ignore_ascii_case("wlast");
            if is_legacy_local_profile
                && profile.base_url.eq_ignore_ascii_case("http://127.0.0.1:9573")
            {
                profile.name = "ai2npu".to_string();
                profile.base_url = "http://localhost:9555".to_string();
            }
        }

        if !self.api_profiles.iter().any(|profile| profile.name.eq_ignore_ascii_case("ai2npu")) {
            self.api_profiles.push(ai2npu_profile());
        }
        if !self.api_profiles.iter().any(|profile| {
            profile.name.eq_ignore_ascii_case("groq")
                || profile.base_url.to_ascii_lowercase().contains("groq.com")
        }) {
            self.api_profiles.push(groq_profile());
        }

        self.api_profiles.sort_by_key(|profile| {
            if profile.name.eq_ignore_ascii_case("ai2npu") {
                (0, profile.name.to_ascii_lowercase())
            } else if profile.name.eq_ignore_ascii_case("groq")
                || profile.base_url.to_ascii_lowercase().contains("groq.com")
            {
                (1, profile.name.to_ascii_lowercase())
            } else {
                (2, profile.name.to_ascii_lowercase())
            }
        });

        if self.active_api_profile_id.is_empty()
            || !self
                .api_profiles
                .iter()
                .any(|profile| profile.id == self.active_api_profile_id)
        {
            self.active_api_profile_id = self
                .api_profiles
                .iter()
                .find(|profile| profile.name.eq_ignore_ascii_case("ai2npu"))
                .map(|profile| profile.id.clone())
                .unwrap_or_else(|| self.api_profiles[0].id.clone());
        }
    }
}

fn ai2npu_profile() -> ApiProfile {
    ApiProfile {
        id: "ai2npu".to_string(),
        name: "ai2npu".to_string(),
        base_url: "http://localhost:9555".to_string(),
        model: String::new(),
        language: String::new(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    }
}

fn groq_profile() -> ApiProfile {
    ApiProfile {
        id: "groq".to_string(),
        name: "groq".to_string(),
        base_url: "https://api.groq.com/openai/".to_string(),
        model: "whisper-large-v3".to_string(),
        language: String::new(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    }
}

fn normalize_hotkey_or(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}
```

Modify `src/lib.rs`:

```rust
pub mod app;
pub mod i18n;
pub mod settings;
```

- [ ] **Step 4: Implement localization constants**

Create `src/i18n.rs`:

```rust
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
```

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test --test settings_tests
cargo test
```

Expected: tests pass.

Commit:

```powershell
git add src tests Cargo.toml
git commit -m "feat: port settings defaults and localization"
```

---

### Task 3: API Endpoint and Client Core

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/endpoints.rs`
- Create: `src/api/models.rs`
- Create: `src/api/transcription.rs`
- Modify: `src/lib.rs`
- Test: unit tests inside `src/api/endpoints.rs`, `src/api/models.rs`, and `src/api/transcription.rs`

- [ ] **Step 1: Write failing API unit tests**

Add module-level tests in `src/api/endpoints.rs`, `src/api/models.rs`, and `src/api/transcription.rs`:

```rust
use super::*;

#[test]
fn endpoint_joins_base_url_and_path() {
    assert_eq!(
        endpoint("https://api.example.com/openai/", "/v1/models").unwrap().as_str(),
        "https://api.example.com/openai/v1/models"
    );
}

#[test]
fn endpoint_rejects_full_audio_endpoint_as_base_url() {
    let error = endpoint(
        "https://api.example.com/v1/audio/transcriptions",
        "/v1/audio/transcriptions",
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("Base URL must not include an audio endpoint"));
}

#[test]
fn translation_error_mentions_model_without_response_body() {
    let message = sanitize_api_error(
        AudioRequestKind::Translation,
        "whisper-large-v3",
        422,
        "Unprocessable Entity",
        r#"{"text":"private recognized user text"}"#,
    );

    assert!(message.contains("422"));
    assert!(message.contains("whisper-large-v3"));
    assert!(message.contains("may not support audio translation"));
    assert!(!message.contains("private recognized user text"));
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test api
```

Expected: fails because the API functions are not implemented.

- [ ] **Step 3: Implement endpoint construction and sanitized errors**

Create `src/api/mod.rs`:

```rust
pub mod endpoints;
pub mod models;
pub mod transcription;
```

Create `src/api/endpoints.rs`:

```rust
use reqwest::Url;

pub fn endpoint(base_url: &str, path: &str) -> anyhow::Result<Url> {
    let base = base_url.trim();
    if base.contains("/v1/audio/transcriptions") || base.contains("/v1/audio/translations") {
        anyhow::bail!("Base URL must not include an audio endpoint");
    }

    let mut normalized = base.trim_end_matches('/').to_string();
    normalized.push('/');
    let url = Url::parse(&normalized)?;
    Ok(url.join(path.trim_start_matches('/'))?)
}
```

Create `src/api/transcription.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRequestKind {
    Transcription,
    Translation,
}

pub fn sanitize_api_error(
    kind: AudioRequestKind,
    model: &str,
    status: u16,
    reason: &str,
    _response_body: &str,
) -> String {
    let mut message = format!("Audio API request failed: {status} {reason}. Model: {model}.");
    if kind == AudioRequestKind::Translation {
        message.push_str(" The selected model may not support audio translation.");
    }
    message
}
```

Create `src/api/models.rs`:

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<ModelInfo>,
}
```

Modify `src/lib.rs`:

```rust
pub mod api;
pub mod app;
pub mod i18n;
pub mod settings;
```

- [ ] **Step 4: Add HTTP client functions**

Extend `src/api/models.rs`:

```rust
use crate::api::endpoints::endpoint;

pub async fn load_models(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
) -> anyhow::Result<Vec<ModelInfo>> {
    let mut request = http.get(endpoint(base_url, "/v1/models")?);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("Models API request failed: {}", status.as_u16());
    }
    Ok(response.json::<ModelsResponse>().await?.data)
}
```

Extend `src/api/transcription.rs`:

```rust
use crate::api::endpoints::endpoint;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AudioTextResponse {
    text: Option<String>,
}

pub async fn send_audio(
    http: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    language: Option<&str>,
    temperature: Option<f64>,
    wav_bytes: Vec<u8>,
    kind: AudioRequestKind,
) -> anyhow::Result<String> {
    let path = match kind {
        AudioRequestKind::Transcription => "/v1/audio/transcriptions",
        AudioRequestKind::Translation => "/v1/audio/translations",
    };
    let file = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("recording.wav")
        .mime_str("audio/wav")?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", model.to_string());
    if kind == AudioRequestKind::Transcription {
        if let Some(language) = language.filter(|value| !value.trim().is_empty()) {
            form = form.text("language", language.to_string());
        }
    }
    if let Some(temperature) = temperature {
        form = form.text("temperature", temperature.to_string());
    }

    let mut request = http.post(endpoint(base_url, path)?).multipart(form);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("HTTP error");
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("{}", sanitize_api_error(kind, model, status.as_u16(), reason, &body));
    }

    let parsed = response.json::<AudioTextResponse>().await?;
    parsed
        .text
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Audio response does not contain a text field."))
}
```

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test
```

Expected: tests pass.

Commit:

```powershell
git add src tests Cargo.toml
git commit -m "feat: port openai compatible audio api core"
```

---

### Task 4: Settings Persistence, Paths, Logging, and DPAPI Secrets

**Files:**
- Create: `src/paths.rs`
- Create: `src/secrets.rs`
- Create: `src/logging.rs`
- Modify: `src/lib.rs`
- Create: `tests/persistence_tests.rs`

- [ ] **Step 1: Write failing persistence tests**

Create `tests/persistence_tests.rs`:

```rust
use std::collections::BTreeMap;
use voiceinsert::paths::AppPaths;
use voiceinsert::settings::AppSettings;

#[test]
fn settings_roundtrip_uses_settings_json() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(temp.path());
    let settings = AppSettings::default().normalized();

    voiceinsert::settings::save_settings(&paths, &settings).unwrap();
    let loaded = voiceinsert::settings::load_settings(&paths).unwrap();

    assert_eq!(loaded.active_profile().name, "ai2npu");
    assert!(paths.settings_path().ends_with("settings.json"));
}

#[test]
fn secret_payload_roundtrip_preserves_profile_keys() {
    let mut keys = BTreeMap::new();
    keys.insert("profile-a".to_string(), "sk-a".to_string());
    keys.insert("profile-b".to_string(), "sk-b".to_string());

    let encoded = voiceinsert::secrets::serialize_keys(&keys).unwrap();
    let decoded = voiceinsert::secrets::deserialize_keys(&encoded).unwrap();

    assert_eq!(decoded, keys);
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --test persistence_tests
```

Expected: fails because paths, persistence, and secrets are missing.

- [ ] **Step 3: Implement app paths**

Create `src/paths.rs`:

```rust
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
        Ok(Self { app_data, local_data })
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
```

Modify `src/lib.rs` to add:

```rust
pub mod logging;
pub mod paths;
pub mod secrets;
```

- [ ] **Step 4: Implement settings load/save**

Append to `src/settings.rs`:

```rust
use crate::paths::AppPaths;
use std::fs;

pub fn load_settings(paths: &AppPaths) -> anyhow::Result<AppSettings> {
    let path = paths.settings_path();
    if !path.exists() {
        return Ok(AppSettings::default().normalized());
    }
    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str::<AppSettings>(&json)?.normalized())
}

pub fn save_settings(paths: &AppPaths, settings: &AppSettings) -> anyhow::Result<()> {
    fs::create_dir_all(paths.app_data_dir())?;
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(paths.settings_path(), json)?;
    Ok(())
}
```

- [ ] **Step 5: Implement secrets payload and DPAPI wrapper**

Create `src/secrets.rs`:

```rust
use std::collections::BTreeMap;

pub fn serialize_keys(keys: &BTreeMap<String, String>) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec(keys)?)
}

pub fn deserialize_keys(bytes: &[u8]) -> anyhow::Result<BTreeMap<String, String>> {
    let text = String::from_utf8(bytes.to_vec())?;
    if text.trim_start().starts_with('{') {
        Ok(serde_json::from_str(&text)?)
    } else {
        let mut keys = BTreeMap::new();
        keys.insert("default".to_string(), text);
        Ok(keys)
    }
}
```

Add Windows DPAPI functions in the same file using `windows::Win32::Security::Cryptography::CryptProtectData` and `CryptUnprotectData`. Keep them behind `#[cfg(windows)]` and return an error on non-Windows.

- [ ] **Step 6: Implement logging setup**

Create `src/logging.rs`:

```rust
use crate::paths::AppPaths;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct LastErrorState {
    inner: Arc<Mutex<Option<String>>>,
}

impl LastErrorState {
    pub fn set(&self, message: impl Into<String>) {
        *self.inner.lock().expect("last error lock poisoned") = Some(message.into());
    }

    pub fn get(&self) -> Option<String> {
        self.inner.lock().expect("last error lock poisoned").clone()
    }
}

pub fn init(paths: &AppPaths, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(paths.logs_dir())?;
    let filter = match level {
        "Debug" => "debug",
        "Warning" => "warn",
        "Error" => "error",
        _ => "info",
    };
    let file_appender = tracing_appender::rolling::daily(paths.logs_dir(), "voiceinsert.log");
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_appender)
        .try_init()
        .ok();
    Ok(())
}
```

- [ ] **Step 7: Run tests and commit**

Run:

```powershell
cargo test --test persistence_tests
cargo test
```

Expected: tests pass.

Commit:

```powershell
git add src tests Cargo.toml
git commit -m "feat: add settings persistence logging and secrets"
```

---

### Task 5: Recording Policy, Audio Buffering, and WAV Encoding

**Files:**
- Create: `src/audio/mod.rs`
- Create: `src/audio/levels.rs`
- Create: `src/audio/recorder.rs`
- Create: `tests/audio_tests.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing audio policy tests**

Create `tests/audio_tests.rs`:

```rust
use voiceinsert::audio::levels::{peak_level_i16_le, should_stop_on_silence};
use voiceinsert::settings::RecordingMode;

#[test]
fn toggle_and_hold_do_not_stop_on_silence() {
    assert!(!should_stop_on_silence(RecordingMode::Toggle));
    assert!(!should_stop_on_silence(RecordingMode::Hold));
}

#[test]
fn silence_timeout_stops_on_silence() {
    assert!(should_stop_on_silence(RecordingMode::SilenceTimeout));
}

#[test]
fn peak_level_reads_signed_16bit_samples() {
    let bytes = [0x00, 0x00, 0xff, 0x7f];
    let level = peak_level_i16_le(&bytes);
    assert!(level > 0.99);
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --test audio_tests
```

Expected: fails because audio modules are missing.

- [ ] **Step 3: Implement audio policy helpers**

Create `src/audio/mod.rs`:

```rust
pub mod levels;
pub mod recorder;
```

Create `src/audio/levels.rs`:

```rust
use crate::settings::RecordingMode;

pub fn should_stop_on_silence(mode: RecordingMode) -> bool {
    matches!(mode, RecordingMode::SilenceTimeout)
}

pub fn peak_level_i16_le(bytes: &[u8]) -> f32 {
    let mut max = 0i32;
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        max = max.max(sample.abs());
    }
    (max as f32 / 32768.0).clamp(0.0, 1.0)
}
```

Modify `src/lib.rs`:

```rust
pub mod audio;
```

- [ ] **Step 4: Implement recorder skeleton**

Create `src/audio/recorder.rs` with a `Recorder` type that:

- lists devices through `cpal::default_host().input_devices()`;
- records mono 16 kHz or device-supported input converted to mono PCM;
- writes WAV to an in-memory `Vec<u8>` through `hound::WavWriter`;
- emits peak levels through a callback or channel;
- returns `Vec<u8>` from `stop`.

Keep this implementation isolated from `app.rs` so tests can continue without microphone access.

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test --test audio_tests
cargo test
```

Expected: deterministic tests pass. Manual microphone behavior is verified later.

Commit:

```powershell
git add src tests Cargo.toml
git commit -m "feat: add audio policy and recorder foundation"
```

---

### Task 6: Hotkey Parsing and Registration

**Files:**
- Create: `src/hotkeys/mod.rs`
- Create: `src/hotkeys/matcher.rs`
- Create: `src/hotkeys/service.rs`
- Create: `tests/hotkey_tests.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing hotkey tests**

Create `tests/hotkey_tests.rs`:

```rust
use voiceinsert::hotkeys::matcher::{normalize_hotkey, validate_pair};

#[test]
fn normalizes_modifier_order() {
    assert_eq!(normalize_hotkey("space+ctrl").unwrap(), "Ctrl+Space");
    assert_eq!(normalize_hotkey("Y+Alt").unwrap(), "Alt+Y");
}

#[test]
fn rejects_hotkeys_without_modifier() {
    assert!(normalize_hotkey("Space").is_err());
}

#[test]
fn rejects_duplicate_transcription_and_translation_hotkeys() {
    assert!(validate_pair("Ctrl+Space", "Ctrl+Space").is_err());
}
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --test hotkey_tests
```

Expected: fails because hotkey modules are missing.

- [ ] **Step 3: Implement hotkey matcher**

Create `src/hotkeys/mod.rs`:

```rust
pub mod matcher;
pub mod service;
```

Create `src/hotkeys/matcher.rs` with normalization for:

- modifiers: `Ctrl`, `Alt`, `Shift`, `Win`;
- keys: letters `A-Z`, digits `0-9`, `Space`, function keys `F1-F24`;
- canonical order: `Ctrl`, `Alt`, `Shift`, `Win`, key;
- at least one modifier required;
- duplicate pair rejected.

- [ ] **Step 4: Implement global hotkey service shell**

Create `src/hotkeys/service.rs` with an interface:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    TranscribePressed,
    TranslatePressed,
    Released,
}

pub trait HotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction>;
}
```

Implement the Windows registration adapter using `global-hotkey` first. If release events are not available, leave a focused `windows` keyboard-hook implementation path in this module.

Modify `src/lib.rs`:

```rust
pub mod hotkeys;
```

- [ ] **Step 5: Run tests and commit**

Run:

```powershell
cargo test --test hotkey_tests
cargo test
```

Expected: tests pass.

Commit:

```powershell
git add src tests Cargo.toml
git commit -m "feat: add hotkey parsing and service foundation"
```

---

### Task 7: Clipboard, Sounds, Tray, and Windows Lifecycle

**Files:**
- Create: `src/clipboard.rs`
- Create: `src/sounds.rs`
- Create: `src/tray.rs`
- Modify: `src/app.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add clipboard adapter**

Create `src/clipboard.rs`:

```rust
pub struct ClipboardInserter {
    pub restore_clipboard: bool,
    pub delay_before_paste_ms: u64,
    pub delay_before_restore_ms: u64,
}
```

Implement:

- read existing clipboard text/data where supported;
- set text through `arboard`;
- call WinAPI `SetForegroundWindow` for the target handle when available;
- send `Ctrl+V` through `SendInput`;
- restore previous clipboard content after the configured delay when possible.

Do not log inserted text.

- [ ] **Step 2: Add sounds**

Create `src/sounds.rs` that embeds `assets/record-start.wav` and `assets/record-stop.wav` with `include_bytes!`. If implementing playback requires an additional crate, use a small focused dependency and keep failures non-fatal.

- [ ] **Step 3: Add tray service**

Create `src/tray.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Idle,
    Recording,
    Transcribing,
    Error,
}
```

Implement localized `Settings` and `Exit` menu creation. Wire menu events to app commands without putting business logic in `tray.rs`.

- [ ] **Step 4: Add single-instance guard**

In `src/app.rs`, implement a Windows named mutex equivalent to the current single-instance behavior. If another instance is already running, exit without starting tray/hotkeys.

- [ ] **Step 5: Manual smoke checks and commit**

Run:

```powershell
cargo build
cargo test
```

Expected: build and tests pass.

Manual later:

- tray icon appears;
- tray menu has Settings and Exit;
- app exits cleanly;
- clipboard paste works into Notepad.

Commit:

```powershell
git add src Cargo.toml assets
git commit -m "feat: add windows shell integration foundation"
```

---

### Task 8: Slint Settings Window and Overlay

**Files:**
- Create: `ui/settings.slint`
- Create: `ui/overlay.slint`
- Modify: `build.rs`
- Modify: `src/app.rs`
- Create: `src/overlay.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create Slint UI files**

Create `ui/settings.slint` with sections:

- General;
- Hotkeys;
- Audio;
- Transcription API;
- Insertion;
- Sounds;
- Logs.

Expose callbacks for:

- save;
- close;
- refresh devices;
- test microphone;
- load models;
- test API connection;
- add/delete profile;
- change transcription hotkey;
- change translation hotkey;
- open logs folder;
- clear logs.

Create `ui/overlay.slint` with properties:

- `status_text`;
- `level`;
- `is_silent`;
- waveform data model or rolling amplitude values.

- [ ] **Step 2: Compile Slint**

Modify `build.rs`:

```rust
fn main() {
    slint_build::compile("ui/settings.slint").expect("failed to compile settings UI");
    slint_build::compile("ui/overlay.slint").expect("failed to compile overlay UI");
}
```

- [ ] **Step 3: Add UI adapters**

Create `src/overlay.rs` with methods:

- `show_recording`;
- `set_level`;
- `set_status`;
- `hide`;
- `reset_waveform`.

In `src/app.rs`, wire settings UI callbacks to settings state, API client, recorder device list, and localization.

- [ ] **Step 4: Run build and commit**

Run:

```powershell
cargo build
cargo test
```

Expected: build and tests pass.

Commit:

```powershell
git add ui src build.rs Cargo.toml
git commit -m "feat: add slint settings and recording overlay"
```

---

### Task 9: Application State Machine Integration

**Files:**
- Modify: `src/app.rs`
- Modify: `src/audio/recorder.rs`
- Modify: `src/api/transcription.rs`
- Modify: `src/clipboard.rs`
- Create: `tests/recording_state_tests.rs`

- [ ] **Step 1: Write state-machine tests**

Create `tests/recording_state_tests.rs` for pure policy:

```rust
use voiceinsert::audio::levels::should_stop_on_silence;
use voiceinsert::settings::RecordingMode;

#[test]
fn silence_policy_matches_spec() {
    assert!(!should_stop_on_silence(RecordingMode::Toggle));
    assert!(!should_stop_on_silence(RecordingMode::Hold));
    assert!(should_stop_on_silence(RecordingMode::SilenceTimeout));
}
```

- [ ] **Step 2: Integrate recording flow**

In `src/app.rs`, implement:

- hotkey pressed starts recording for transcription or translation;
- hotkey release stops recording only in `Hold`;
- toggle press stops active recording;
- silence timeout stops only in `SilenceTimeout`;
- max duration stops all modes;
- stop flow changes overlay to transcribing/translating, calls API, changes overlay to inserting, then calls clipboard insertion;
- all errors play error sound, update last error, log safe technical details, hide overlay, and return to idle.

- [ ] **Step 3: Run tests and manual checks**

Run:

```powershell
cargo test
cargo build --release
```

Manual:

- `Ctrl+Space` records transcription;
- `Alt+Y` records translation;
- `Hold` stops on release;
- `Toggle` ignores silence;
- overlay resets between recordings.

- [ ] **Step 4: Commit**

```powershell
git add src tests
git commit -m "feat: integrate voice insertion state machine"
```

---

### Task 10: Installer, Scripts, CI, and Documentation

**Files:**
- Modify: `installer/VoiceInsert.iss`
- Modify: `scripts/build.ps1`
- Modify: `scripts/test.ps1`
- Modify: `scripts/package.ps1`
- Modify: `scripts/publish.ps1`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`
- Modify: `README_ru.md`
- Create: `SECURITY.md` if still missing

- [ ] **Step 1: Update scripts**

Replace .NET commands with Cargo equivalents:

- build: `cargo build --configuration equivalent` where Release maps to `cargo build --release`;
- test: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`;
- package: `cargo build --release`, then Inno Setup against `target\release`.

- [ ] **Step 2: Update installer**

Modify `installer/VoiceInsert.iss`:

- `PublishDir` points to `..\target\release`;
- package `VoiceInsert.exe`, assets, and required runtime files;
- keep `AppId=strokinkv.VoiceInsert`;
- keep `DefaultDirName={userappdata}\VoiceInsert`;
- keep `OutputBaseFilename=VoiceInsertSetup`;
- keep quiet uninstall registration.

- [ ] **Step 3: Update CI**

Change `.github/workflows/ci.yml` to:

```yaml
name: CI

on:
  push:
    branches:
      - main
      - master
  pull_request:

jobs:
  test:
    name: Build and test
    runs-on: windows-latest

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Format check
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Test
        run: cargo test

      - name: Build
        run: cargo build --release
```

- [ ] **Step 4: Update documentation**

Update README and README_ru:

- Rust/Slint implementation notes;
- Cargo build/test commands;
- installer build instructions;
- preserve user-facing usage text.

Add `SECURITY.md` with a short vulnerability reporting policy and privacy reminder.

- [ ] **Step 5: Commit**

```powershell
git add installer scripts .github README.md README_ru.md SECURITY.md CHANGELOG.md
git commit -m "chore: update packaging ci and docs for rust"
```

---

### Task 11: Remove C#/.NET Project

**Files:**
- Delete: `VoiceInsert.slnx`
- Delete: `Directory.Build.props`
- Delete: `NuGet.Config`
- Delete: `src/VoiceInsert.App/`
- Delete: `tests/VoiceInsert.App.Tests/`
- Delete: `.dotnet/`
- Delete: `.nuget/`
- Delete: `TestResults/`
- Modify: `.gitignore`

- [ ] **Step 1: Verify Rust replacement builds before deletion**

Run:

```powershell
cargo test
cargo build --release
```

Expected: pass.

- [ ] **Step 2: Remove old .NET files**

Delete the old C# project and local .NET state directories. Keep docs, installer, scripts, assets under the new layout.

- [ ] **Step 3: Update ignore rules**

Update `.gitignore` to ignore:

- `target/`;
- `artifacts/`;
- `.appdata/`;
- `.localappdata/`;
- local installer output;
- editor and OS files.

Remove ignore rules that only apply to obsolete `.NET` local state if no longer needed.

- [ ] **Step 4: Run final checks and commit**

Run:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
git status --short
```

Expected: Rust checks pass; git status only shows intended deletions/modifications before commit.

Commit:

```powershell
git add -A
git commit -m "chore: remove dotnet implementation"
```

---

### Task 12: Final Verification and Smoke Checklist

**Files:**
- Create: `docs/rust-smoke-checklist.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Create smoke checklist**

Create `docs/rust-smoke-checklist.md` with checkboxes for:

- app starts and creates one tray icon;
- second instance exits;
- settings opens from tray;
- tray menu localizes in RU/EN;
- default profiles are `ai2npu` then `groq`;
- model loading calls `/v1/models`;
- transcription calls `/v1/audio/transcriptions`;
- translation calls `/v1/audio/translations`;
- translation failure logs model and support hint without response body text;
- `Ctrl+Space` starts transcription;
- `Alt+Y` starts translation;
- `Toggle`, `Hold`, and `Silence timeout` behave per spec;
- overlay waveform appears and resets;
- clipboard insertion works in Notepad;
- logs omit API key, audio, recognized text, and response body;
- installer silent install works;
- `winget uninstall "VoiceInsert" --silent` path is registered.

- [ ] **Step 2: Run final automated verification**

Run:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
.\scripts\package.ps1
```

Expected: all commands pass and installer is produced at `artifacts\installer\VoiceInsertSetup.exe`.

- [ ] **Step 3: Run manual smoke checks**

Run through `docs/rust-smoke-checklist.md` on Windows 11. Mark each observed result.

- [ ] **Step 4: Commit verification docs**

```powershell
git add docs/rust-smoke-checklist.md CHANGELOG.md
git commit -m "docs: add rust rewrite smoke checklist"
```
