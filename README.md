# System Color Refresh

Native Windows utility in Rust for refreshing and managing system colors after sleep and resume.

## MVP

- Read `Hilight` and `HotTrackingColor` from `HKCU\Control Panel\Colors`
- Apply them to the current session with `SetSysColors`
- Broadcast `WM_SYSCOLORCHANGE` and `WM_SETTINGCHANGE`
- CLI entrypoint: `syscolor apply`

## Layout

- `src/color.rs` - RGB and `COLORREF` conversion
- `src/registry.rs` - Registry access
- `src/win32.rs` - Unsafe Win32 wrapper layer
- `src/system_colors.rs` - Business logic
- `src/broadcast.rs` - Notification broadcast helpers
- `src/cli/` - CLI parsing and command dispatch

## Run

```bash
cargo run -- apply
```

## Notes

- Unsafe code is isolated in `src/win32.rs`.
- Backup and restore are scaffolded for the next iteration.