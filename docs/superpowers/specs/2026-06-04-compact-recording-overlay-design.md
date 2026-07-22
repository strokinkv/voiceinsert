# Compact Recording Overlay Design

## Goal

Replace the large recording popup with a compact floating status window that shows only the current operation and a running waveform. The overlay should be readable at a glance and occupy only the space needed for the waveform.

## Layout

- Window size: compact fixed-size Slint window near the Windows clock area.
- Status: centered at the top of the overlay.
- Waveform: single framed strip below the status.
- No separate dot indicator.
- No `live` or `silent` labels.
- Waveform bars are close together and centered in the waveform strip.

## Behavior

- Status text is localized through the selected UI language:
  - Russian: `Запись`, `Распознавание`, `Вставка`.
  - English: `Recording`, `Transcribing`, `Inserting`.
- Waveform uses a rolling history of amplitude samples.
- Waveform propagation is intentionally slow enough to be readable.
- Silence still affects recording state logic, but it does not recolor the full waveform to gray.
- Each new recording resets the waveform history.

## Implementation Notes

- `RecordingOverlay` owns only visual state required for display: status text, wave step, and wave bar values.
- `UiController` owns the current level and rolling waveform buffer.
- `AppRuntime` advances the overlay from the existing UI tick.
- The overlay receives localized status strings from `crate::i18n::texts`.

## Verification

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- Manual visual check of the overlay preview or installed app.
