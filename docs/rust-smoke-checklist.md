# Rust Rewrite Smoke Checklist

Use this checklist on Windows 11 after antivirus handling for Cargo `target/` is settled.

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `cargo test` passes.
- [ ] `cargo build --release` creates `target\release\VoiceInsert.exe`.
- [ ] `.\scripts\package.ps1` creates `artifacts\installer\VoiceInsertSetup.exe`.
- [ ] App starts and creates one tray icon.
- [ ] Starting a second instance exits without creating another tray icon.
- [ ] Settings opens from tray.
- [ ] Tray menu localizes in Russian and English.
- [ ] Default profiles are `ai2npu` then `groq`.
- [ ] `ai2npu` points to `http://localhost:9555`.
- [ ] Model loading calls `GET {base_url}/v1/models`.
- [ ] Transcription calls `POST {base_url}/v1/audio/transcriptions`.
- [ ] Translation calls `POST {base_url}/v1/audio/translations`.
- [ ] Translation failure logs model and support hint without response body text.
- [ ] `Ctrl+Space` starts transcription.
- [ ] `Alt+Y` starts translation.
- [ ] `Toggle`, `Hold`, and `Silence timeout` behave per spec.
- [ ] Overlay waveform appears and resets between recordings.
- [ ] Clipboard insertion works in Notepad.
- [ ] Logs omit API key, audio, recognized text, and response body.
- [ ] Silent install works with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`.
- [ ] `winget uninstall "VoiceInsert" --silent` path is registered through quiet uninstall.
