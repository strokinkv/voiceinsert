# Contributing

## Development Setup

Requirements:

- Windows 11
- .NET 10 SDK
- Inno Setup 6, only for installer builds

Useful commands:

```powershell
.\scripts\build.ps1
.\scripts\test.ps1
.\scripts\package.ps1
```

## Guidelines

- Keep user speech text, audio, and API keys out of logs and test fixtures.
- Keep UI strings localized in `SettingsTexts`.
- Prefer focused changes with tests for shared behavior.
- Run `.\scripts\test.ps1` before opening a pull request.

## Installer Changes

When changing publishing or installer behavior, verify:

```powershell
.\scripts\package.ps1
.\scripts\smoke-install.ps1
```
