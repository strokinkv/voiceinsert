# VoiceInsert

![VoiceInsert](docs/assets/VoiceInsert.png)

VoiceInsert is a Windows 11 background utility. It records speech from a microphone, sends WAV audio to an OpenAI-compatible Audio API, and inserts the returned text into the active window through the clipboard.

Russian documentation: [README_ru.md](README_ru.md)

Technical specification: [docs/technical-specification.md](docs/technical-specification.md)

## Installation

Download and run `VoiceInsertSetup.exe`.

Silent install:

```powershell
.\artifacts\installer\VoiceInsertSetup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART
```

## Development

VoiceInsert is being rewritten as a Rust Windows application with a Slint UI.

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
.\scripts\package.ps1
```

## How to Use

1. Start VoiceInsert. The app appears in the system tray.
2. Open `Settings` from the tray context menu.
3. Select an API profile, enter an API key if needed, load models, and select a model.
4. Place the cursor in the target window and press `Ctrl+Space` to recognize speech.

During recording, VoiceInsert shows a compact floating window with a centered localized status and a scrolling signal amplitude waveform. After the API response is received, the text is inserted into the active window through the clipboard.

## Default API Profiles

VoiceInsert creates two API profiles on first launch:

- `ai2npu`, first and primary profile: `http://localhost:9555`
- `groq`: `https://api.groq.com/openai/`, without an API key

Model discovery:

```text
GET {base_url}/v1/models
```

Transcription:

```text
POST {base_url}/v1/audio/transcriptions
```

Each API profile has one model field. VoiceInsert sends audio only to the transcription endpoint.

## Current Behavior

- The tray context menu is localized and contains `Settings` / `Exit`.
- Default transcription hotkey: `Ctrl+Space`.
- Recording modes: toggle, hold, silence timeout.
- In toggle and hold modes, silence does not stop recording; only max recording duration applies.
- The recording overlay uses a compact waveform-only indicator; it does not show separate `live` or `silent` labels.
- Text insertion uses the clipboard.
- API keys are protected with Windows DPAPI per profile.
- Audio and recognized text are not saved to disk.
- Logs do not contain audio, recognized text, or API keys.
- Settings use a single-screen card layout without tabs or scrolling. API language input is not shown; audio requests default to Russian (`ru`) as the input language.
- VoiceInsert always records from the current system default microphone.
- The `Logs` section shows the logs folder, log level, and last error for the current app session.

## Privacy

- VoiceInsert does not save audio to disk.
- Recognized text is not stored after insertion.
- Logs do not include audio, recognized text, or API keys.
- API keys are stored through Windows DPAPI separately for each profile.
- Audio is sent only to the API endpoint configured in the selected profile.
- With the default `ai2npu` profile, requests go to the local address `http://localhost:9555`.

## Troubleshooting

- **No text is inserted:** make sure the target app is focused, increase the paste delay in `Settings > Insertion`, and try disabling clipboard restore for apps with custom clipboard handling.
- **Hotkey does not work:** choose a hotkey with at least one modifier key and avoid shortcuts already used by the active app. `Ctrl+Space` can conflict with IDEs and input methods.
- **Microphone is missing or silent:** refresh devices, select the full microphone name, and use the microphone level test before recording.
- **API returns 401:** check the API key for the active profile. Keys are stored separately for each profile.
- **API returns 404 or 422:** verify that the base URL does not include `/v1/audio/...` and that the server supports OpenAI-compatible `/v1/models` and `/v1/audio/transcriptions` endpoints.
- **Installer cannot close the app:** exit VoiceInsert from the tray menu before installing or uninstalling.

## Limitations

- Windows 11 only.
- Requires an OpenAI-compatible Audio API with `/v1/audio/transcriptions` and `/v1/models` endpoints.
- Text insertion uses the clipboard, so behavior can depend on the active application.
- Microphone input is recorded through the Rust audio backend and converted to mono 16 kHz PCM WAV before API upload.
