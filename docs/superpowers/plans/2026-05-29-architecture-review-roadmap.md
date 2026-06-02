# Architecture Review Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Close the new review findings and then reduce the main Rust runtime architecture risk without changing the user-facing VoiceInsert workflow.

**Architecture:** First land two small correctness fixes and one migration fix while the code is still easy to verify. Then split `src/app.rs` into focused modules, replace ad-hoc background threads with runtime-owned async tasks, and move timing/audio/UI maintenance issues into isolated follow-up tasks.

**Tech Stack:** Rust 2024, Slint 1.14, Tokio, reqwest, cpal, tray-icon, Windows APIs through the `windows` crate.

---

## Review Triage

Confirmed and should be fixed before architecture work:

- `src/app.rs:609`: `completion.map_err(|error| error.to_string())` bypasses `log_error_message()`.
- `src/app.rs:246` and delete-profile flow near `src/app.rs:215`: active API profile changes can leave `self.http` using the previous profile timeout.
- `src/settings.rs`: old settings files with top-level `temperature` / `requestTimeoutSeconds` and no `apiProfiles` lose those values after the recent cleanup.

Valid architecture backlog:

- Split `src/app.rs` into smaller runtime, state, background, and command modules.
- Replace background `thread + Handle::block_on()` with `tokio::spawn()`.
- Make silence timing use audio-frame duration, not wall-clock callback gaps.
- Add CI coverage and dependency audit jobs.
- Add E2E test with a mock OpenAI-compatible server.

Lower-priority cleanup:

- `src/sounds.rs`: avoid recreating cpal host/device/config for every sound.
- `src/tray.rs`: remove the `TrayService` wrapper if it stays internal-only.
- `src/ui.rs`: add explicit settings-window close handling if Slint default behavior becomes risky.
- `src/overlay.rs`: replace the double pointer cast in `SystemParametersInfoW`.
- Merge duplicate hotkey / recording-state integration tests.
- Split `ui/settings.slint` after behavior stabilizes.

Rejected or downgraded:

- The `old_profile` “dangling reference” item is technically incorrect. `old_profile` is an owned clone. Keep only as readability debt if touched during app split.
- `Rc<RefCell<AppRuntime>>` is acceptable for the current single-threaded Slint event loop. Revisit only after the app split, not as an urgent bug.

---

### Task 1: Sanitize Voice Insertion Completion Errors

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app.rs`

- [x] **Step 1: Add the failing regression test**

Add this test inside `#[cfg(test)] mod tests` in `src/app.rs`:

```rust
#[test]
fn voice_completion_error_uses_sanitized_root_cause() {
    let error = anyhow::anyhow!("clipboard failed\nprivate recognized text").context("outer context");

    assert_eq!(
        super::voice_completion_error(error),
        "clipboard failed"
    );
}
```

- [x] **Step 2: Run the focused test and verify it fails**

Run:

```powershell
cargo test app::tests::voice_completion_error_uses_sanitized_root_cause -- --exact
```

Expected: compile failure or test failure because `voice_completion_error` does not exist.

- [x] **Step 3: Add the helper and use it in the completion path**

In `src/app.rs`, add:

```rust
fn voice_completion_error(error: anyhow::Error) -> String {
    log_error_message(&error)
}
```

Change the background completion send from:

```rust
completion.map_err(|error| error.to_string())
```

to:

```rust
completion.map_err(voice_completion_error)
```

- [x] **Step 4: Verify the focused test passes**

Run:

```powershell
cargo test app::tests::voice_completion_error_uses_sanitized_root_cause -- --exact
```

Expected: `1 passed`.

- [x] **Step 5: Run app tests**

Run:

```powershell
cargo test app::tests
```

Expected: all `app::tests` pass.

---

### Task 2: Rebuild HTTP Client When Active Profile Changes

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app.rs`

- [x] **Step 1: Extract the HTTP client builder**

Add near other app helpers:

```rust
fn http_client_for_profile(profile: &ApiProfile) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(profile.request_timeout_seconds))
        .build()?)
}
```

Replace both inline `reqwest::Client::builder()` blocks in `initialize()` and `save_settings_from_ui()` with this helper.

- [x] **Step 2: Add a unit test for the profile-timeout comparison helper**

Add a small helper first if needed:

```rust
fn profile_requires_http_rebuild(old_profile: &ApiProfile, new_profile: &ApiProfile) -> bool {
    old_profile.request_timeout_seconds != new_profile.request_timeout_seconds
}
```

Add this test:

```rust
#[test]
fn profile_timeout_change_requires_http_rebuild() {
    let mut old_profile = ApiProfile::default();
    old_profile.request_timeout_seconds = 30;
    let mut new_profile = old_profile.clone();
    new_profile.request_timeout_seconds = 120;

    assert!(super::profile_requires_http_rebuild(&old_profile, &new_profile));
}
```

- [x] **Step 3: Rebuild the client in API profile switch/delete/add flows**

After `self.settings = self.settings.clone().normalized();` in `UiCommand::AddApiProfile`, `UiCommand::DeleteApiProfile`, and `UiCommand::SelectApiProfile`, set:

```rust
self.http = http_client_for_profile(self.settings.active_profile())?;
```

Keep the existing `queue_model_load_for_active_profile()?` calls.

- [x] **Step 4: Verify tests**

Run:

```powershell
cargo test app::tests::profile_timeout_change_requires_http_rebuild -- --exact
cargo test app::tests
```

Expected: focused test passes and app tests remain green.

---

### Task 3: Migrate Legacy Top-Level Profile Settings

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\settings.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\tests\settings_tests.rs`

- [x] **Step 1: Add a failing legacy migration test**

Add to `tests/settings_tests.rs`:

```rust
#[test]
fn legacy_top_level_profile_values_migrate_when_profiles_are_missing() {
    let json = r#"{
        "settingsVersion": 7,
        "activeApiProfileId": "",
        "temperature": 0.7,
        "requestTimeoutSeconds": 45
    }"#;

    let settings = voiceinsert::settings::settings_from_json(json).unwrap();

    assert_eq!(settings.active_profile().temperature, 0.7);
    assert_eq!(settings.active_profile().request_timeout_seconds, 45);
}
```

- [x] **Step 2: Run the focused test and verify it fails**

Run:

```powershell
cargo test legacy_top_level_profile_values_migrate_when_profiles_are_missing --test settings_tests -- --exact
```

Expected: compile failure because `settings_from_json` does not exist, or assertion failure because legacy values are ignored.

- [x] **Step 3: Add a JSON-loading helper**

In `src/settings.rs`, change `load_settings()` to call a new helper:

```rust
pub fn settings_from_json(json: &str) -> anyhow::Result<AppSettings> {
    let value = serde_json::from_str::<serde_json::Value>(json)?;
    let mut settings = serde_json::from_value::<AppSettings>(value.clone())?.normalized();
    apply_legacy_profile_values(&mut settings, &value);
    Ok(settings.normalized())
}
```

Then update `load_settings()`:

```rust
let json = fs::read_to_string(path)?;
settings_from_json(&json)
```

- [x] **Step 4: Apply legacy profile values only when old files had no usable profiles**

Add:

```rust
fn apply_legacy_profile_values(settings: &mut AppSettings, value: &serde_json::Value) {
    let had_profiles = value
        .get("apiProfiles")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|profiles| !profiles.is_empty());
    if had_profiles {
        return;
    }

    let Some(profile) = settings.api_profiles.first_mut() else {
        return;
    };

    if let Some(temperature) = value.get("temperature").and_then(serde_json::Value::as_f64) {
        profile.temperature = temperature;
    }
    if let Some(timeout) = value
        .get("requestTimeoutSeconds")
        .and_then(serde_json::Value::as_u64)
    {
        profile.request_timeout_seconds = timeout;
    }
}
```

- [x] **Step 5: Verify settings tests**

Run:

```powershell
cargo test --test settings_tests
```

Expected: all settings tests pass.

---

### Task 4: Split Runtime State From `src/app.rs`

**Files:**
- Move: `C:\Users\strokin\projects\voiceinsert\src\app.rs` to `C:\Users\strokin\projects\voiceinsert\src\app\mod.rs`
- Create: `C:\Users\strokin\projects\voiceinsert\src\app\state.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\lib.rs`

- [x] **Step 1: Move `src/app.rs` into an app module directory**

Use a non-destructive move command:

```powershell
New-Item -ItemType Directory -Force src\app
Move-Item -LiteralPath src\app.rs -Destination src\app\mod.rs
```

Keep `pub mod app;` unchanged in `src/lib.rs`; Rust will load `src/app/mod.rs`.

- [x] **Step 2: Create `src/app/state.rs` with state machine types**

Move these items from `src/app/mod.rs` to `src/app/state.rs`:

Move the complete existing definitions for `OperationState`, `VoiceInsertState`, `StateCommand`, `log_error_message()`, and `sanitize_log_text()` exactly as they exist in `src/app/mod.rs` at execution time. Make `log_error_message()` and `sanitize_log_text()` `pub(super)` if only `src/app/background.rs` needs them, or `pub` if tests outside the module need direct access.

Also move the state-machine unit tests from `src/app/mod.rs` into `src/app/state.rs`.

- [x] **Step 3: Wire the module from `src/app/mod.rs`**

At the top of `src/app/mod.rs`, add:

```rust
mod state;

use state::{
    OperationState, StateCommand, VoiceInsertState, log_error_message, sanitize_log_text,
};
```

Remove the moved definitions from `src/app/mod.rs`.

- [x] **Step 4: Verify module split**

Run:

```powershell
cargo fmt --all
cargo test app::state::tests
cargo test app::tests
```

Expected: state tests and remaining app tests pass.

---

### Task 5: Split Background Tasks From `src/app.rs`

**Files:**
- Create: `C:\Users\strokin\projects\voiceinsert\src\app\background.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app\mod.rs`

- [x] **Step 1: Move background types**

Move these items to `src/app/background.rs`:

Move the complete existing definitions for `BackgroundEvent`, `AudioTask`, `spawn_voice_insert_task()`, and `spawn_load_models_task()` exactly as they exist in `src/app/mod.rs` at execution time. Keep `BackgroundEvent` public to `src/app/mod.rs`; keep `AudioTask` fields public only where `AppRuntime` constructs it.

Make fields public only where `AppRuntime` needs to construct values.

- [x] **Step 2: Import them from `src/app/mod.rs`**

Add:

```rust
mod background;

use background::{AudioTask, BackgroundEvent, spawn_load_models_task, spawn_voice_insert_task};
```

- [x] **Step 3: Verify**

Run:

```powershell
cargo fmt --all
cargo test app::tests
cargo clippy --all-targets -- -D warnings
```

Expected: app tests pass and clippy stays clean.

---

### Task 6: Replace Background Threads With Tokio Tasks

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app\background.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app\mod.rs`

- [x] **Step 1: Add a focused test for completion sanitization in background module**

In `src/app/background.rs` tests:

```rust
#[test]
fn completion_error_is_sanitized_before_event() {
    let error = anyhow::anyhow!("private line\nsecond line").context("outer context");

    assert_eq!(voice_completion_error(error), "private line");
}
```

- [x] **Step 2: Change task spawning to use the Tokio handle**

Replace the `std::thread::Builder::new()` wrapper in `spawn_voice_insert_task()` with a Tokio task:

```rust
handle.spawn(async move {
    let completion = async {
        let AudioTask {
            base_url,
            api_key,
            model,
            language,
            temperature,
            wav_bytes,
            kind,
            target_window,
        } = task;

        let text = send_audio(
            &http,
            SendAudioRequest {
                base_url: &base_url,
                api_key: &api_key,
                model: &model,
                language: language.as_deref(),
                temperature: Some(temperature),
                wav_bytes,
                kind,
            },
        )
        .await?;
        let _ = events.send(BackgroundEvent::VoiceInsertionStarted);
        clipboard.insert_text(&text, target_window).await
    }
    .await;

    let _ = events.send(BackgroundEvent::VoiceInsertionComplete(
        completion.map_err(voice_completion_error),
    ));
});
```

For models:

```rust
handle.spawn(async move {
    match load_models(&http, &base_url, &api_key).await {
        Ok(models) => {
            let model_ids = models.into_iter().map(|model| model.id).collect();
            let _ = events.send(BackgroundEvent::ModelsLoaded {
                profile_id,
                models: model_ids,
            });
        }
        Err(error) => {
            let _ = events.send(BackgroundEvent::Error(log_error_message(&error)));
        }
    }
});
```

- [x] **Step 3: Simplify return types**

If no fallible operation remains in `spawn_voice_insert_task()` and `spawn_load_models_task()`, change them from `anyhow::Result<()>` to `()`, and remove `?` from call sites in `src/app/mod.rs`.

- [x] **Step 4: Verify**

Run:

```powershell
cargo test app::background::tests
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all tests pass and clippy stays clean.

---

### Task 7: Make Silence Timing Use Audio Frame Duration

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\audio\recorder.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app\mod.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\tests\audio_tests.rs`

- [x] **Step 1: Add a test for frame duration calculation**

Add to `tests/audio_tests.rs`:

```rust
#[test]
fn frame_duration_uses_sample_count_and_capture_rate() {
    assert_eq!(
        voiceinsert::audio::recorder::frame_duration_ms(480, 48_000),
        10
    );
    assert_eq!(
        voiceinsert::audio::recorder::frame_duration_ms(160, 16_000),
        10
    );
}
```

- [x] **Step 2: Implement the helper**

In `src/audio/recorder.rs`:

```rust
pub fn frame_duration_ms(mono_sample_count: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    ((mono_sample_count as u64) * 1000 / u64::from(sample_rate)).max(1)
}
```

- [x] **Step 3: Extend recorder level callback**

Change recorder callback from:

```rust
F: FnMut(f32) + Send + 'static,
```

to:

```rust
F: FnMut(f32, u64) + Send + 'static,
```

In `capture_samples()`, after downmixing:

```rust
let delta_ms = frame_duration_ms(mono.len(), sample_rate);
on_level(level, delta_ms);
```

Pass `stream_config.sample_rate.0` into each `capture_samples()` call.

- [x] **Step 4: Remove wall-clock timing from app runtime**

In `src/app/mod.rs`, replace:

```rust
let mut last_level_at = Instant::now();
recorder.start(move |level| {
    let now = Instant::now();
    let delta_ms = now.duration_since(last_level_at).as_millis() as u64;
    last_level_at = now;
    let _ = send_level(&level_tx, level, delta_ms);
})?;
```

with:

```rust
recorder.start(move |level, delta_ms| {
    let _ = send_level(&level_tx, level, delta_ms);
})?;
```

Remove the now-unused `Instant` import.

- [x] **Step 5: Verify audio and app tests**

Run:

```powershell
cargo test --test audio_tests
cargo test app::state::tests
cargo clippy --all-targets -- -D warnings
```

Expected: all pass.

---

### Task 8: Add CI Dependency Audit and Coverage

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\.github\workflows\ci.yml`

- [x] **Step 1: Add cargo-audit install/cache step**

Add after Rust setup:

```yaml
      - name: Install cargo-audit
        uses: taiki-e/install-action@cargo-audit
```

- [x] **Step 2: Add dependency audit step**

Add after clippy:

```yaml
      - name: Dependency audit
        run: cargo audit
```

- [x] **Step 3: Add llvm-cov coverage install and run**

Add:

```yaml
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Coverage
        run: cargo llvm-cov --workspace --lcov --output-path lcov.info
```

- [x] **Step 4: Upload coverage artifact**

Add:

```yaml
      - name: Upload coverage artifact
        uses: actions/upload-artifact@v4
        with:
          name: coverage-lcov
          path: lcov.info
```

- [x] **Step 5: Verify YAML shape locally**

Run:

```powershell
cargo fmt --all
cargo test
```

Expected: local commands pass. The workflow itself must be validated by GitHub Actions on push/PR.

---

### Task 9: Add Mock-Server E2E Coverage

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\Cargo.toml`
- Create: `C:\Users\strokin\projects\voiceinsert\tests\e2e_api_tests.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\api\transcription.rs` only if testability requires a public helper

- [x] **Step 1: Add dev dependency**

In `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
wiremock = "0.6"
```

- [x] **Step 2: Add E2E API test**

Create `tests/e2e_api_tests.rs`:

```rust
use voiceinsert::api::transcription::{AudioRequestKind, SendAudioRequest, send_audio};
use voiceinsert::audio::recorder::encode_wav_mono_16khz_i16;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn transcribes_wav_against_openai_compatible_mock() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello from mock"
        })))
        .mount(&server)
        .await;

    let http = reqwest::Client::new();
    let wav_bytes = encode_wav_mono_16khz_i16(&[0, 100, -100]).unwrap();

    let text = send_audio(
        &http,
        SendAudioRequest {
            base_url: &server.uri(),
            api_key: "",
            model: "openai/whisper-large-v3-turbo",
            language: None,
            temperature: Some(0.2),
            wav_bytes,
            kind: AudioRequestKind::Transcription,
        },
    )
    .await
    .unwrap();

    assert_eq!(text, "hello from mock");
}
```

- [x] **Step 3: Verify**

Run:

```powershell
cargo test --test e2e_api_tests
```

Expected: mock-server test passes.

---

### Task 10: Improve Clipboard Diagnostics

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\clipboard.rs`

- [x] **Step 1: Replace silent clipboard read ignore**

Change:

```rust
clipboard.get_text().ok()
```

to:

```rust
match clipboard.get_text() {
    Ok(text) => Some(text),
    Err(error) => {
        tracing::debug!(%error, "clipboard did not contain readable text");
        None
    }
}
```

- [x] **Step 2: Verify**

Run:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all pass.

---

### Task 11: Low-Risk Cleanup Batch

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\overlay.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\autostart.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\tests\audio_tests.rs`
- Delete: `C:\Users\strokin\projects\voiceinsert\tests\recording_state_tests.rs`

- [x] **Step 1: Clean overlay pointer cast**

In `src/overlay.rs`, replace:

```rust
Some(&mut rect as *mut _ as *mut _)
```

with:

```rust
Some((&mut rect as *mut RECT).cast())
```

- [x] **Step 2: Improve Win32 error formatting**

In `src/autostart.rs`, replace:

```rust
anyhow::bail!("{context}: Win32 error {}", result.0)
```

with:

```rust
anyhow::bail!("{context}: {result:?}")
```

- [x] **Step 3: Merge the recording-state integration test**

Move this test from `tests/recording_state_tests.rs` into `tests/audio_tests.rs`:

```rust
#[test]
fn silence_policy_matches_spec() {
    assert!(!should_stop_on_silence(RecordingMode::Toggle));
    assert!(!should_stop_on_silence(RecordingMode::Hold));
    assert!(should_stop_on_silence(RecordingMode::SilenceTimeout));
}
```

Then delete `tests/recording_state_tests.rs`.

- [x] **Step 4: Verify cleanup**

Run:

```powershell
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all pass.

---

### Task 12: Sound Service Caching

**Files:**
- Modify: `C:\Users\strokin\projects\voiceinsert\src\sounds.rs`
- Modify: `C:\Users\strokin\projects\voiceinsert\src\app\mod.rs`

- [x] **Step 1: Change `SoundService` to hold decoded sound samples**

Change:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundService {
    pub enabled: bool,
}
```

to:

```rust
#[derive(Debug, Clone)]
pub struct SoundService {
    pub enabled: bool,
    start: WavSamples,
    stop: WavSamples,
    error: WavSamples,
}
```

Add:

```rust
impl SoundService {
    pub fn new(enabled: bool) -> anyhow::Result<Self> {
        Ok(Self {
            enabled,
            start: decode_wav(sound_bytes(SoundKind::Start))?,
            stop: decode_wav(sound_bytes(SoundKind::Stop))?,
            error: decode_wav(sound_bytes(SoundKind::Error))?,
        })
    }
}
```

- [x] **Step 2: Use cached decoded samples in `play()`**

Inside `play()`, replace:

```rust
let samples = decode_wav(sound_bytes(kind))?;
```

with:

```rust
let samples = match kind {
    SoundKind::Start => self.start.clone(),
    SoundKind::Stop => self.stop.clone(),
    SoundKind::Error => self.error.clone(),
};
```

Keep device/config lookup in `play()` unless measurements prove that stream creation is a user-visible delay.

- [x] **Step 3: Update runtime initialization**

In `src/app/mod.rs`, replace:

```rust
let sounds = SoundService {
    enabled: settings.enable_sounds,
};
```

with:

```rust
let sounds = SoundService::new(settings.enable_sounds)?;
```

- [x] **Step 4: Verify**

Run:

```powershell
cargo test sounds::tests
cargo test
```

Expected: all pass.

---

### Task 13: Settings UI Component Split

**Files:**
- Create: `C:\Users\strokin\projects\voiceinsert\ui\components.slint`
- Modify: `C:\Users\strokin\projects\voiceinsert\ui\settings.slint`

- [x] **Step 1: Move reusable components**

Move these component definitions from `settings.slint` to `components.slint`:

Move the complete existing component definitions for `Label`, `ValueText`, `Field`, `FieldRow`, `ComboRow`, `NavItem`, `SmallButton`, `TinyButton`, `ToggleRow`, `OptionButton`, `PageTitle`, `SectionTitle`, and `Section`. Add `export` to each moved component so `settings.slint` can import it. Do not change dimensions, colors, text sizes, or bindings during the split.

- [x] **Step 2: Import components from `settings.slint`**

At the top of `settings.slint`, add:

```slint
import {
    Label, ValueText, FieldRow, ComboRow, NavItem, SmallButton, TinyButton,
    ToggleRow, OptionButton, PageTitle, SectionTitle, Section
} from "./components.slint";
```

Keep `import { ComboBox } from "std-widgets.slint";` in `components.slint`, not in `settings.slint`.

- [x] **Step 3: Verify Slint build**

Run:

```powershell
cargo build
cargo test ui::tests
```

Expected: build succeeds and UI tests pass.

---

## Final Verification

After all tasks in this plan:

```powershell
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
git diff --check
```

Then reinstall and launch:

```powershell
$procs = Get-Process VoiceInsert -ErrorAction SilentlyContinue
if ($procs) {
    $procs | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}
.\scripts\package.ps1
.\scripts\deploy-local.ps1
$exe = Join-Path $env:APPDATA 'VoiceInsert\VoiceInsert.exe'
Start-Process -FilePath $exe -WindowStyle Hidden
```
