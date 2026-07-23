# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-23

### Added
- **`syscolor apply`** — Read `Hilight` and `HotTrackingColor` from the Windows registry and apply them to the current session via `SetSysColors`.
- **`syscolor reload`** — Alias for `apply`; re-applies registry colors without reboot.
- **`syscolor get <highlight|hot-tracking>`** — Print the current live Windows system color value.
- **`syscolor set [target] <color|R G B>`** — Write a new color to the registry and immediately apply it. Accepts preset names (`pink`, `green`, `blue`, `violet`, `orange`) or explicit `R G B` components.
- **`syscolor dump`** — Print all managed system colors currently stored in the registry.
- **`syscolor backup <path>`** — Export current registry colors to a JSON file for later restoration.
- **`syscolor restore <path>`** — Import colors from a backup JSON file and apply them.
- **WM_SYSCOLORCHANGE** and **WM_SETTINGCHANGE** broadcast after every apply so all running applications pick up the new colors immediately.
- **Windows MSI installer** (`syscolor-0.1.0-x86_64.msi`) — installs `syscolor.exe` to `Program Files` and optionally adds it to the system `PATH`.

[0.1.0]: https://github.com/NV-Phong/syscolor/releases/tag/v0.1.0
