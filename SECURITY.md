# Security Policy

## Reporting a Vulnerability

Report security issues privately through GitHub Security Advisories when available, or by opening a GitHub issue that does not include secrets, API keys, audio, recognized text, or local settings.

## Privacy Expectations

VoiceInsert must not log or persist:

- API keys;
- recorded audio;
- recognized text;
- full request or response bodies that may contain user text.

API keys are stored locally with Windows DPAPI for the current Windows user.
