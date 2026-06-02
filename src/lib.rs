//! Core library for the VoiceInsert Windows speech-to-text tray application.
//!
//! The crate exposes the runtime, settings, audio, API, tray, UI, and Windows
//! integration modules used by the `VoiceInsert.exe` binary.

pub mod api;
pub mod app;
pub mod audio;
pub mod autostart;
pub mod clipboard;
pub mod hotkeys;
pub mod i18n;
pub mod logging;
pub mod overlay;
pub mod paths;
pub mod secrets;
pub mod settings;
pub mod sounds;
pub mod tray;
pub mod ui;
