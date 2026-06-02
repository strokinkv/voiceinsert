# Single-Window Settings Design

## Goal

Redesign the VoiceInsert settings window as a single-screen settings panel with no tabs, no sidebar, no scrolling, and no persistent header or status bar. The window should fit all settings in one `1360x800` desktop layout.

The design must keep the settings readable while reducing navigation cost. The user should see every configurable value at once.

## Window Structure

- Window size: `1360x800`.
- No sidebar navigation.
- No tab controls.
- No app title/header text.
- No active-profile line in the top area.
- No bottom status bar.
- No permanent status badge.
- All content sits inside a masonry-style settings panel.
- The masonry layout is orderly and grid-aligned, not a free-form dashboard.
- Cards may have different widths and heights based on their content.
- Cards have no group titles and no icons.
- Individual field labels remain explicit.

The normal state contains only the setting cards. Errors appear as a toast overlay above the content and must not move or resize the card layout.

## Visual Style

Use a new light blue-graphite style rather than the current green-teal palette.

Concept image: [2026-06-02-single-window-settings-concept.png](2026-06-02-single-window-settings-concept.png).

- Background: pale blue-gray.
- Cards: white or near-white.
- Text: graphite/dark slate.
- Accent: muted blue for focus, selected dropdown state, and enabled toggles.
- Borders: thin blue-gray.
- Radius: compact, around `6px` to `8px`.
- Shadows: subtle, only enough to separate cards from the background.

The UI should feel like a serious Windows 11 utility: dense, quiet, and functional.

## Layout

The layout is driven by field count and field width, not by perceived importance.

Recommended card placement:

- A wide API card in the upper-left or left column.
- A medium recording/hotkeys card near the API card.
- Compact application and logs cards on the right.
- An insertion card in the lower area where it best fits the masonry grid.

The API card can be wider because it contains long values such as `Base URL`, `API key`, and `Model`. This is a content fit decision, not a visual priority signal.

## Field Presentation

- Labels appear above fields.
- Text inputs remain plain text fields.
- Numeric values remain normal text fields.
- Binary settings remain toggle controls.
- All option sets become dropdown controls.
- Small `?` hint markers appear next to complex labels and show explanations on hover.
- Do not use segmented option buttons.
- Do not use steppers for numeric fields.
- `API key` remains a normal visible text field, not a password field.

Tooltip candidates:

- `Temperature`
- `Timeout`
- `Порог тишины`
- `Таймаут тишины`
- `Макс. длительность`
- `Задержка перед вставкой`
- `Задержка перед восстановлением`
- `Уровень логов`

## Cards And Fields

API card:

- `Активный профиль` dropdown.
- Compact `+` and `-` buttons for adding and deleting API profiles.
- `Имя профиля` text field.
- `Base URL` text field.
- `API key` visible text field.
- `Model` dropdown.
- `Temperature` text field.
- `Timeout` text field.

Recording/hotkeys card:

- `Клавиша транскрибации` text field.
- `Клавиша перевода` text field.
- `Режим записи` dropdown.
- `Порог тишины` text field.
- `Таймаут тишины` text field.
- `Макс. длительность` text field.

Insertion card:

- `Восстанавливать буфер` toggle.
- `Задержка перед вставкой` text field.
- `Задержка перед восстановлением` text field.

Application card:

- `Запускать вместе с Windows` toggle.
- `Звуковое оповещение` toggle.
- `Язык интерфейса` dropdown.

Logs card:

- `Открыть папку логов` button.
- `Уровень логов` dropdown.

The logs folder path is not shown.

## Removed Settings UI

Remove the API profile `Language / Язык распознавания` field from the settings window.

When sending audio API requests, use `ru` as the default input language for both transcription and translation if no explicit language is configured internally. This default is a behavior change and should be covered in the implementation plan.

## Dropdown Values

Existing option-button groups become dropdowns:

- UI language: `Русский`, `English`.
- Recording mode: `Переключатель`, `Удержание`, `Тишина`.
- Log level: `Инфо`, `Отладка`, `Предупреждения`, `Ошибки`.

Model and active profile remain dropdowns as they already are.

## Error Handling

Replace persistent status text with toast-style messages:

- Toast appears only when there is an error or a user-visible problem.
- Toast overlays the content near the upper-right area.
- Toast does not shift the layout.
- Routine autosave success does not need a permanent visible status.

If later implementation needs non-error feedback, use the same toast mechanism sparingly.

## Implementation Notes

The likely Slint changes are:

- Replace `selected-page` navigation with a single content layout.
- Remove `NavItem` usage from the settings window.
- Add vertical field components with label-above-field layout.
- Add dropdown row/card components for former `OptionButton` groups.
- Add hover hint support for selected labels.
- Add a toast component driven by existing status/error state or a new UI property.
- Remove display and editing of `language` from `SettingsWindow`.

The likely Rust changes are:

- Update `SettingsEdit` and UI bindings to stop reading the removed language field from the settings window.
- Preserve existing profile language data if needed for backwards compatibility, but do not expose it in the UI.
- Ensure transcription and translation requests default to `ru` as input language when no explicit language is configured.
- Keep profile add/delete/select behavior unchanged.
- Keep logs-folder open command unchanged, but remove the visible folder path from UI.

## Verification

Recommended checks:

- Slint compile through `cargo build`.
- Settings behavior tests for profile selection and settings persistence.
- API request tests verifying default `language=ru` for transcription and translation.
- Visual verification at `1360x800` that all cards fit without scrollbars.
- Manual hover check for tooltip markers.
- Manual toast check that errors do not shift the card layout.
