# Changelog

## 2.0.0 - 2026-05-29

- Replaced the application with the Rust + Slint implementation.
- Changed the default local API profile to `ai2npu` at `http://localhost:9555`.
- Removed obsolete project artifacts and stale documentation from the previous implementation.

## Unreleased

- No unreleased changes yet.

## 1.1.0

- Improved recording lifecycle reliability: stale max-duration timers can no longer stop a newer recording session.
- Prevented a second VoiceInsert instance from continuing startup when the app is already running.
- Added controller dependency interfaces and expanded unit test coverage for toggle, hold, silence timeout, API failure, empty audio, and clipboard insertion paths.
- Added API base URL validation and a dedicated API profile health-check command for the settings window.
- Made API clients use per-request authorization headers instead of mutating shared client defaults.
- Added hotkey validation and normalization to reject unsafe or unknown shortcuts and avoid duplicate transcription/translation hotkeys.
- Improved clipboard restoration by preserving the full clipboard data object instead of only Unicode text.
- Strengthened CI with warnings-as-errors, formatting checks, dependency audit, coverage collection, and coverage artifact upload.
- Added GitHub release workflow for packaged installer artifacts.
- Centralized version metadata and passed it to the Inno Setup package build.
- Added installer metadata links and troubleshooting sections in English and Russian documentation.

## 1.0.0

- Initial Windows 11 tray app.
- Global hotkeys for transcription and English speech translation.
- OpenAI-compatible audio API profiles.
- Clipboard insertion with optional clipboard restore.
- Recording overlay with scrolling amplitude waveform.
- Russian and English settings UI.
- DPAPI-protected API keys per profile.
- Inno Setup installer with silent install and uninstall support.
