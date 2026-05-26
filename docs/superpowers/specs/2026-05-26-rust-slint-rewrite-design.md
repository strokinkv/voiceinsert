# VoiceInsert Rust + Slint Rewrite Design

## Goal

Rewrite VoiceInsert as a Rust-only Windows 11 tray utility, replacing the current C# WPF implementation while preserving the user-visible behavior defined in `docs/technical-specification.md`.

## Scope

This rewrite replaces the existing `.slnx`, `.csproj`, C# source, XAML UI, and xUnit tests with a Rust workspace. The application remains a Windows desktop utility named `VoiceInsert.exe`, installed to `%APPDATA%\VoiceInsert`, with settings in `%APPDATA%\VoiceInsert\settings.json` and logs in `%LOCALAPPDATA%\VoiceInsert\Logs`.

The rewrite targets feature parity with the current 1.1.0 behavior:

- system tray app with localized `Settings` and `Exit` menu items;
- global transcription hotkey `Ctrl+Space`;
- global speech-to-English translation hotkey `Alt+Y`;
- recording modes `Toggle`, `Hold`, and `Silence timeout`;
- WAV audio recording in memory only;
- OpenAI-compatible audio API profiles;
- DPAPI-protected API keys per profile;
- clipboard-based insertion with optional clipboard restore;
- compact settings window;
- recording overlay with waveform and status text;
- Inno Setup installer with silent install and uninstall support.

## Architecture

The Rust version will use one Cargo workspace with a primary desktop binary crate. Slint provides the settings and overlay UI. Rust modules own all system integration and business logic.

Proposed structure:

```text
Cargo.toml
src/
  main.rs
  app.rs
  settings.rs
  secrets.rs
  clipboard.rs
  tray.rs
  overlay.rs
  logging.rs
  i18n.rs
  api/
    mod.rs
    endpoints.rs
    models.rs
    transcription.rs
  audio/
    mod.rs
    recorder.rs
    devices.rs
    levels.rs
  hotkeys/
    mod.rs
    matcher.rs
    service.rs
ui/
  settings.slint
  overlay.slint
assets/
  VoiceInsert.ico
  record-start.wav
  record-stop.wav
installer/
  VoiceInsert.iss
tests/
```

The UI layer sends commands to the Rust application state and receives plain data models. The core modules do not depend on Slint so that settings, API behavior, hotkey parsing, and recording policy can be tested without starting the UI.

## Technology Choices

- UI: `slint`
- Async runtime: `tokio`
- HTTP and multipart requests: `reqwest`
- JSON settings: `serde`, `serde_json`
- Logging: `tracing`, `tracing-appender`
- Audio capture: `cpal`
- WAV encoding: `hound`
- Windows APIs: `windows`
- Clipboard support: `arboard` plus WinAPI `SendInput` for paste
- Tray and menu: `tray-icon` if it satisfies requirements, otherwise direct WinAPI through `windows`
- Global hotkeys: `global-hotkey` if it satisfies press/release semantics, otherwise direct WinAPI through `windows`
- Tests: Rust unit and integration tests through `cargo test`
- Installer: keep Inno Setup and update it to package `target/release/VoiceInsert.exe`

## Components

### Application Orchestration

`app.rs` owns startup and lifecycle:

- initialize paths, logging, settings, secrets, audio, API clients, tray, hotkeys, and UI;
- ensure a single running instance;
- open the settings window from tray;
- start, stop, transcribe, translate, and insert according to the active recording mode;
- switch tray and overlay states between idle, recording, transcribing, inserting, and error.

### Settings

`settings.rs` defines the Rust equivalent of the current `AppSettings` and `ApiProfile` models.

It must preserve these defaults:

- active profile `ai2npu`;
- `ai2npu` base URL `http://localhost:9555`;
- `groq` base URL `https://api.groq.com/openai/`;
- `groq` model `whisper-large-v3`;
- transcription hotkey `Ctrl+Space`;
- translation hotkey `Alt+Y`;
- UI language Russian by default.

It also handles:

- migration from old local `Default` or `wlast` profiles to `ai2npu`;
- normalization of hotkeys;
- clamping numeric settings;
- one shared `Model` field for transcription and translation;
- keeping prompt fields out of the UI.

### Secrets

`secrets.rs` stores API keys separately from `settings.json` using Windows DPAPI for the current user.

The first implementation should try to read the existing C# DPAPI file format at `%APPDATA%\VoiceInsert\api-key.dpapi`, including the legacy single-key format and the JSON dictionary keyed by profile id. On save, the Rust version may write a Rust-owned JSON dictionary encrypted with DPAPI.

### API

The API module implements:

- `GET {base_url}/v1/models`;
- `POST {base_url}/v1/audio/transcriptions`;
- `POST {base_url}/v1/audio/translations`.

Requests use `multipart/form-data` with WAV bytes as `file`, the selected `model`, optional transcription `language`, and optional `temperature`.

API error handling must not log recognized text, full response bodies, API keys, audio, or multipart request bodies. Translation endpoint failures must include the selected model and a diagnostic note that the model may not support audio translation.

### Audio

The audio module records microphone input to WAV bytes in memory.

Required behavior:

- list input devices with full Windows CoreAudio friendly names where possible;
- keep enough device identity to map UI selection back to the capture device reliably;
- provide a short microphone-level test;
- emit level updates for overlay waveform and silence detection;
- clear audio buffers after each operation;
- protect against excessive recording duration and buffer growth.

### Hotkeys

The hotkey module registers global hotkeys and normalizes user-captured combinations.

Required behavior:

- `Ctrl+Space` starts transcription by default;
- `Alt+Y` starts speech-to-English translation by default;
- reject empty, unsafe, unknown, or duplicate hotkeys;
- support press and release handling for `Hold` mode.

If a crate cannot reliably expose press/release semantics on Windows, this module should use direct WinAPI registration and keyboard hooks.

### Clipboard Insertion

`clipboard.rs` implements insertion through the clipboard only:

- optionally preserve previous clipboard content;
- set recognized text;
- focus the captured target window when available;
- send `Ctrl+V` through `SendInput`;
- optionally restore previous clipboard content after the configured delay.

Insertion reliability is more important than clipboard restoration.

### Tray

`tray.rs` provides a localized tray icon and menu:

- `Settings`;
- `Exit`.

It also supports state changes for idle, recording, transcribing, and short-lived error indication.

### UI

`ui/settings.slint` implements the settings window with the same sections as the technical specification:

- General;
- Hotkeys;
- Audio;
- Transcription API;
- Insertion;
- Sounds;
- Logs.

The window must keep a compact fixed layout, field-level tooltips, top navigation, profile add/delete controls, one model field, and a bottom status row.

`ui/overlay.slint` implements the floating recording overlay with:

- waveform amplitude over time;
- silence indication;
- status text;
- reset waveform state on every new recording.

### Logging

`logging.rs` writes logs to `%LOCALAPPDATA%\VoiceInsert\Logs` and tracks the last error for the current process.

Logs may include:

- event time;
- application state;
- recording duration;
- audio byte size;
- endpoint or base URL without secrets;
- HTTP status code;
- error type;
- stack/backtrace where available;
- request duration;
- successful insertion event without inserted text.

Logs must not include:

- recognized text;
- audio data;
- API keys;
- full request bodies;
- full response bodies that may contain user text.

## Migration Plan

Work happens on branch `rewrite-rust-slint`.

The implementation order is:

1. Create Cargo workspace and a minimal Windows binary named `VoiceInsert.exe`.
2. Port settings, API profile defaults, localization, and normalization.
3. Port API endpoint validation, model loading, transcription, translation, and sanitized errors.
4. Implement DPAPI secrets and legacy secret migration.
5. Implement audio capture, WAV encoding, input-device list, level test, max duration, and silence detection.
6. Implement global hotkeys, tray menu, clipboard insertion, and sounds.
7. Implement Slint settings window and overlay.
8. Update installer, scripts, GitHub Actions, README, README_ru, and release docs.
9. Remove C# project files, XAML, xUnit tests, and .NET build scripts.
10. Verify with `cargo test`, `cargo fmt --check`, `cargo clippy`, release build, installer build, and Windows smoke testing.

## Compatibility

The Rust version should preserve the existing settings path and as much of the existing `settings.json` schema as practical. It may rewrite JSON formatting after save.

The Rust version should read existing DPAPI secrets where possible. If full write-format compatibility is not practical, it must support read-migration from the old format and save in the new encrypted format.

Installer identity remains:

- app name: `VoiceInsert`;
- package id: `strokinkv.VoiceInsert`;
- install directory: `%APPDATA%\VoiceInsert`;
- installer output: `VoiceInsertSetup.exe`;
- quiet uninstall registration for winget.

## Testing Strategy

Rust tests cover deterministic behavior:

- settings defaults and migration;
- API profile ordering;
- hotkey normalization and duplicate rejection;
- endpoint construction;
- API error sanitization;
- recording mode policy;
- JSON settings read/write;
- DPAPI secret read/write where Windows CI allows it.

Windows-only integrations use smoke checks:

- app starts and shows tray icon;
- settings opens from tray;
- hotkeys register and trigger recording;
- hold mode stops on release;
- toggle mode ignores silence timeout;
- silence timeout mode stops on silence;
- overlay appears and resets waveform;
- clipboard insertion works in Notepad;
- installer silent install and uninstall work.

## Acceptance Criteria

The rewrite is complete when:

- the C# application has been fully replaced by a Rust application;
- `VoiceInsert.exe` builds in release mode on Windows;
- user-facing behavior matches `docs/technical-specification.md`;
- deterministic logic is covered by Rust tests;
- Windows integration behavior is covered by a smoke checklist;
- installer packaging preserves the current app identity;
- documentation and scripts no longer reference .NET build commands as the primary path;
- no logs, tests, docs, or fixtures contain API keys, recognized text, user audio, or local settings.
