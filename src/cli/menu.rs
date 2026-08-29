use std::collections::BTreeMap;
use std::fs;
use std::io::{stdout, Write};
use std::path::{Path, PathBuf};

use crossterm::{
    cursor::{Hide, Show},
    event::{read, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use crate::{
    color::Color,
    constants::SystemColorSlot,
    errors::{Error, Result},
    registry,
    system_colors::{SystemColorManager, SystemColors},
};

// ── Theme Colors from User Fastfetch Specification ──────────────────────────
const COLOR_BORDER: (u8, u8, u8) = (82, 90, 107);      // #525A6B (Slate Gray)
const COLOR_PRESETS: (u8, u8, u8) = (139, 212, 156);   // #8BD49C (Mint / Pastel Green)
const COLOR_ACTIONS: (u8, u8, u8) = (231, 199, 135);   // #E7C787 (Amber / Gold)
const COLOR_STATUS: (u8, u8, u8) = (125, 166, 216);    // #7DA6D8 (Soft Blue)
const COLOR_CONTROLS: (u8, u8, u8) = (215, 160, 201);  // #D7A0C9 (Lavender / Pink)

const CARD_WIDTH: usize = 68;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetMode {
    Both,
    Highlight,
    HotTracking,
}

impl TargetMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "Both (Highlight + Hot-Tracking)",
            Self::Highlight => "Highlight Only (Hilight)",
            Self::HotTracking => "Hot-Tracking Only (HotTrackingColor)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Both => "Both",
            Self::Highlight => "Highlight",
            Self::HotTracking => "Hot-Tracking",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Both => Self::Highlight,
            Self::Highlight => Self::HotTracking,
            Self::HotTracking => Self::Both,
        }
    }
}

pub struct PresetItem {
    pub name: &'static str,
    pub color: Color,
    pub shortcut: Option<char>,
}

fn get_presets() -> Vec<PresetItem> {
    vec![
        PresetItem {
            name: "Pink",
            color: Color::new(255, 153, 153),
            shortcut: Some('1'),
        },
        PresetItem {
            name: "Green",
            color: Color::new(172, 217, 119),
            shortcut: Some('2'),
        },
        PresetItem {
            name: "Blue",
            color: Color::new(114, 176, 246),
            shortcut: Some('3'),
        },
        PresetItem {
            name: "Violet",
            color: Color::new(208, 153, 255),
            shortcut: Some('4'),
        },
        PresetItem {
            name: "Orange",
            color: Color::new(236, 187, 110),
            shortcut: Some('5'),
        },
        PresetItem {
            name: "Nord Frost",
            color: Color::new(136, 192, 208),
            shortcut: Some('6'),
        },
    ]
}

#[derive(Clone, Debug)]
enum Toast {
    Success(String),
    Info(String),
    Error(String),
}

// ── RAII Terminal Guard ─────────────────────────────────────────────────────
struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().map_err(Error::Io)?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All)).map_err(Error::Io)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = stdout();
        let _ = execute!(out, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

// ── Main Interactive Select Menu Loop ───────────────────────────────────────
pub fn run_interactive_menu() -> Result<()> {
    let _guard = TerminalGuard::new()?;
    let manager = SystemColorManager;

    let mut selected_index: usize = 0;
    let mut target_mode = TargetMode::Both;
    let mut toast: Option<Toast> = None;

    let presets = get_presets();
    let total_preset_count = presets.len();

    // Total actionable items in main menu:
    // 0..total_preset_count: Presets
    // total_preset_count: Custom Color
    // total_preset_count + 1: Toggle Target Mode
    // total_preset_count + 2: Apply Registry Colors
    // total_preset_count + 3: Reload & Sync Session
    // total_preset_count + 4: Backup Registry to JSON
    // total_preset_count + 5: Restore Registry from JSON
    // total_preset_count + 6: Dump & Inspect Registry Values
    // total_preset_count + 7: Exit
    let total_items = total_preset_count + 8;

    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            // Query live state
            let live_colors = manager.get().ok();
            let registry_colors = manager.dump_registry().ok();

            render_main_screen(
                selected_index,
                target_mode,
                &presets,
                live_colors,
                registry_colors,
                &toast,
            )?;
            needs_redraw = false;
        }

        // Wait blocking for user input event (0% CPU, ZERO flicker)
        let event = read().map_err(Error::Io)?;
        if let Event::Key(key) = event {
            // Ignore key release events
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }

            needs_redraw = true;

            // Handle hotkeys & navigation
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('q') | KeyCode::Esc => break,

                // Arrow Navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected_index == 0 {
                        selected_index = total_items - 1;
                    } else {
                        selected_index -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected_index = (selected_index + 1) % total_items;
                }
                KeyCode::Home => selected_index = 0,
                KeyCode::End => selected_index = total_items - 1,

                // Target Mode Toggle
                KeyCode::Tab | KeyCode::Char('t') => {
                    target_mode = target_mode.next();
                    toast = Some(Toast::Info(format!("Target set to: {}", target_mode.label())));
                }

                // Direct Preset Shortcuts (1-6)
                KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                    let idx = (c as usize) - ('1' as usize);
                    if idx < presets.len() {
                        let preset = &presets[idx];
                        match apply_color_to_target(&manager, target_mode, preset.color) {
                            Ok(()) => {
                                toast = Some(Toast::Success(format!(
                                    "Applied preset '{}' {} ({}) to {}",
                                    preset.name,
                                    preset.color.to_hex(),
                                    preset.color,
                                    target_mode.short_label()
                                )));
                            }
                            Err(e) => toast = Some(Toast::Error(format!("Apply error: {e}"))),
                        }
                    }
                }

                // Direct Action Shortcuts
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    match manager.apply() {
                        Ok(()) => {
                            toast = Some(Toast::Success("Applied registry accent colors & broadcasted changes!".into()));
                        }
                        Err(e) => toast = Some(Toast::Error(format!("Failed to apply: {e}"))),
                    }
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    match manager.reload() {
                        Ok(()) => {
                            toast = Some(Toast::Success("Reloaded and synchronized system colors!".into()));
                        }
                        Err(e) => toast = Some(Toast::Error(format!("Failed to reload: {e}"))),
                    }
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    // Open Custom Color Input modal
                    if let Some(custom_color) = prompt_custom_color()? {
                        match apply_color_to_target(&manager, target_mode, custom_color) {
                            Ok(()) => {
                                toast = Some(Toast::Success(format!(
                                    "Applied custom color {} ({}) to {}",
                                    custom_color.to_hex(),
                                    custom_color,
                                    target_mode.short_label()
                                )));
                            }
                            Err(e) => toast = Some(Toast::Error(format!("Failed to set custom color: {e}"))),
                        }
                    }
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    // Backup prompt
                    if let Some(backup_path) = prompt_backup_filename()? {
                        match backup_registry_to_file(&backup_path) {
                            Ok(()) => {
                                toast = Some(Toast::Success(format!(
                                    "Saved backup to '{}'",
                                    backup_path.display()
                                )));
                            }
                            Err(e) => toast = Some(Toast::Error(format!("Backup failed: {e}"))),
                        }
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    // Restore modal
                    if let Some(path) = prompt_restore_file(&manager)? {
                        toast = Some(Toast::Success(format!("Restored from '{}'", path.display())));
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    // Dump inspection screen
                    show_registry_dump_screen()?;
                }

                // Execute Selected Item on Enter / Space
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if selected_index < total_preset_count {
                        // Preset selected
                        let preset = &presets[selected_index];
                        match apply_color_to_target(&manager, target_mode, preset.color) {
                            Ok(()) => {
                                toast = Some(Toast::Success(format!(
                                    "Applied preset '{}' {} ({}) to {}",
                                    preset.name,
                                    preset.color.to_hex(),
                                    preset.color,
                                    target_mode.short_label()
                                )));
                            }
                            Err(e) => toast = Some(Toast::Error(format!("Apply error: {e}"))),
                        }
                    } else {
                        let action_idx = selected_index - total_preset_count;
                        match action_idx {
                            0 => {
                                // Custom Color
                                if let Some(custom_color) = prompt_custom_color()? {
                                    match apply_color_to_target(&manager, target_mode, custom_color) {
                                        Ok(()) => {
                                            toast = Some(Toast::Success(format!(
                                                "Applied custom color {} ({}) to {}",
                                                custom_color.to_hex(),
                                                custom_color,
                                                target_mode.short_label()
                                            )));
                                        }
                                        Err(e) => toast = Some(Toast::Error(format!("Failed to set custom color: {e}"))),
                                    }
                                }
                            }
                            1 => {
                                // Toggle Target Mode
                                target_mode = target_mode.next();
                                toast = Some(Toast::Info(format!("Target set to: {}", target_mode.label())));
                            }
                            2 => {
                                // Apply Registry Colors
                                match manager.apply() {
                                    Ok(()) => {
                                        toast = Some(Toast::Success("Applied registry accent colors to live session!".into()));
                                    }
                                    Err(e) => toast = Some(Toast::Error(format!("Failed to apply: {e}"))),
                                }
                            }
                            3 => {
                                // Reload Session
                                match manager.reload() {
                                    Ok(()) => {
                                        toast = Some(Toast::Success("Reloaded and synchronized system colors!".into()));
                                    }
                                    Err(e) => toast = Some(Toast::Error(format!("Failed to reload: {e}"))),
                                }
                            }
                            4 => {
                                // Backup to JSON
                                if let Some(backup_path) = prompt_backup_filename()? {
                                    match backup_registry_to_file(&backup_path) {
                                        Ok(()) => {
                                            toast = Some(Toast::Success(format!(
                                                "Saved backup to '{}'",
                                                backup_path.display()
                                            )));
                                        }
                                        Err(e) => toast = Some(Toast::Error(format!("Backup failed: {e}"))),
                                    }
                                }
                            }
                            5 => {
                                // Restore from JSON
                                if let Some(path) = prompt_restore_file(&manager)? {
                                    toast = Some(Toast::Success(format!("Restored from '{}'", path.display())));
                                }
                            }
                            6 => {
                                // Dump & Inspect Registry Values
                                show_registry_dump_screen()?;
                            }
                            7 => {
                                // Exit
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                _ => {}
            }
        } else if let Event::Resize(_, _) = event {
            needs_redraw = true;
        }
    }

    Ok(())
}

fn apply_color_to_target(
    manager: &SystemColorManager,
    target: TargetMode,
    color: Color,
) -> Result<()> {
    match target {
        TargetMode::Both => manager.set_all(color),
        TargetMode::Highlight => manager.set(SystemColorSlot::Highlight, color),
        TargetMode::HotTracking => manager.set(SystemColorSlot::HotTracking, color),
    }
}

// ── Flicker-Free Rendering Functions ────────────────────────────────────────

fn render_main_screen(
    selected_index: usize,
    target_mode: TargetMode,
    presets: &[PresetItem],
    live_colors: Option<SystemColors>,
    registry_colors: Option<SystemColors>,
    toast: &Option<Toast>,
) -> Result<()> {
    let mut lines = Vec::new();

    // Title spacing
    lines.push(String::new());

    // ── SECTION 1: PRESET ACCENT COLORS ──────────────────────────────────────
    lines.push(render_box_header("PRESET ACCENT COLORS", COLOR_PRESETS, CARD_WIDTH));

    for (i, preset) in presets.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == presets.len() - 1;
        let is_selected = selected_index == i;

        let shortcut_str = match preset.shortcut {
            Some(c) => format!("[{c}]"),
            None => "   ".into(),
        };

        let swatch_block = format!("{}██\x1b[0m", preset.color.ansi_fg());
        let hex = preset.color.to_hex();
        let rgb_str = format!("({:3}, {:3}, {:3})", preset.color.r, preset.color.g, preset.color.b);

        let content = format!(
            "{} {:<12} {:<8} {}  {}",
            shortcut_str, preset.name, hex, rgb_str, swatch_block
        );

        lines.push(render_box_item(
            is_first,
            is_last,
            &format!("P{}", i + 1),
            &content,
            COLOR_PRESETS,
            CARD_WIDTH,
            is_selected,
        ));
    }
    lines.push(render_box_footer(CARD_WIDTH));

    lines.push(String::new());

    // ── SECTION 2: ACTIONS & CONFIGURATION ───────────────────────────────────
    lines.push(render_box_header("ACTIONS & CONFIGURATION", COLOR_ACTIONS, CARD_WIDTH));

    let action_items = [
        (
            "CUSTOM",
            "[C] Custom Color... (HEX / RGB component input)".into(),
            presets.len() + 0,
        ),
        (
            "TARGET",
            format!("[T] Target Slot: [ {} ]", target_mode.label()),
            presets.len() + 1,
        ),
        (
            "APPLY",
            "[A] Apply Registry Colors (Win32 SetSysColors + Broadcast)".into(),
            presets.len() + 2,
        ),
        (
            "RELOAD",
            "[R] Reload & Synchronize Live Session".into(),
            presets.len() + 3,
        ),
        (
            "BACKUP",
            "[B] Backup Registry Colors to JSON File".into(),
            presets.len() + 4,
        ),
        (
            "RESTORE",
            "[S] Restore Registry Colors from JSON File".into(),
            presets.len() + 5,
        ),
        (
            "DUMP",
            "[D] Dump & Inspect All Registry Color Entries".into(),
            presets.len() + 6,
        ),
        (
            "EXIT",
            "[Q] Exit syscolor".into(),
            presets.len() + 7,
        ),
    ];

    for (i, (key, label, idx)) in action_items.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == action_items.len() - 1;
        let is_selected = selected_index == *idx;

        lines.push(render_box_item(
            is_first,
            is_last,
            key,
            label,
            COLOR_ACTIONS,
            CARD_WIDTH,
            is_selected,
        ));
    }
    lines.push(render_box_footer(CARD_WIDTH));

    lines.push(String::new());

    // ── SECTION 3: LIVE SYSTEM STATUS ────────────────────────────────────────
    lines.push(render_box_header("LIVE SYSTEM STATUS", COLOR_STATUS, CARD_WIDTH));

    let live_hi = live_colors.map(|c| c.highlight).unwrap_or(Color::new(0, 0, 0));
    let live_hot = live_colors.map(|c| c.hot_tracking).unwrap_or(Color::new(0, 0, 0));
    let reg_hi = registry_colors.map(|c| c.highlight).unwrap_or(Color::new(0, 0, 0));
    let reg_hot = registry_colors.map(|c| c.hot_tracking).unwrap_or(Color::new(0, 0, 0));

    let is_synced = live_hi == reg_hi && live_hot == reg_hot;

    let hi_content = format!(
        "Live Hilight:      {} ({:3}, {:3}, {:3})  {}████\x1b[0m",
        live_hi.to_hex(), live_hi.r, live_hi.g, live_hi.b, live_hi.ansi_fg()
    );
    let hot_content = format!(
        "Live HotTracking:  {} ({:3}, {:3}, {:3})  {}████\x1b[0m",
        live_hot.to_hex(), live_hot.r, live_hot.g, live_hot.b, live_hot.ansi_fg()
    );
    let sync_content: String = if is_synced {
        "\x1b[38;2;139;212;156mSYNCHRONIZED\x1b[0m (Live Win32 matches Registry)".to_string()
    } else {
        "\x1b[38;2;249;215;47mOUT OF SYNC\x1b[0m (Press [A] to apply registry colors)".to_string()
    };

    lines.push(render_box_item(true, false, "HILIGHT", &hi_content, COLOR_STATUS, CARD_WIDTH, false));
    lines.push(render_box_item(false, false, "HOTLIGHT", &hot_content, COLOR_STATUS, CARD_WIDTH, false));
    lines.push(render_box_item(false, true, "STATUS", &sync_content, COLOR_STATUS, CARD_WIDTH, false));
    lines.push(render_box_footer(CARD_WIDTH));

    lines.push(String::new());

    // ── SECTION 4: CONTROLS & SHORTCUTS ──────────────────────────────────────
    lines.push(render_box_header("CONTROLS & SHORTCUTS", COLOR_CONTROLS, CARD_WIDTH));
    // Simplified navigation help: remove duplicate/alternate keys
    lines.push(render_box_item(
        true,
        false,
        "NAV",
        "[↑/↓] Navigate  •  [Enter/Space] Select",
        COLOR_CONTROLS,
        CARD_WIDTH,
        false,
    ));
    // Remove duplicated exit hint (Exit already explained in Actions)
    let keys_hint = format!(
        "[1-{}] Presets • [C] Custom • [A] Apply • [R] Reload",
        presets.len()
    );
    lines.push(render_box_item(
        false,
        true,
        "KEYS",
        &keys_hint,
        COLOR_CONTROLS,
        CARD_WIDTH,
        false,
    ));
    lines.push(render_box_footer(CARD_WIDTH));

    // ── PALETTE CIRCLES (Fastfetch Aesthetic) ────────────────────────────────
    lines.push(String::new());
    lines.push(render_palette_dots());

    // ── TOAST NOTIFICATION BAR ───────────────────────────────────────────────
    if let Some(t) = toast {
        lines.push(String::new());
        match t {
            Toast::Success(msg) => {
                lines.push(format!(
                    "  \x1b[38;2;139;212;156m[OK]\x1b[0m  {}",
                    msg
                ));
            }
            Toast::Info(msg) => {
                lines.push(format!(
                    "  \x1b[38;2;125;166;216m[INFO]\x1b[0m  {}",
                    msg
                ));
            }
            Toast::Error(msg) => {
                lines.push(format!(
                    "  \x1b[38;2;255;100;100m[ERR]\x1b[0m  {}",
                    msg
                ));
            }
        }
    }

    // Atomic double buffer write with Synchronized Output (DEC Mode 2026)
    flush_lines_to_terminal(&lines)
}

fn flush_lines_to_terminal(lines: &[String]) -> Result<()> {
    let mut buffer = String::with_capacity(8192);

    // \x1b[?2026h: Begin Synchronized Output (Prevents terminal screen tearing / flickering)
    // \x1b[H: Move cursor to home (top-left 0,0)
    buffer.push_str("\x1b[?2026h\x1b[H");

    for line in lines {
        buffer.push_str(line);
        buffer.push_str("\x1b[K\r\n"); // Clear to end of line & newline
    }

    // \x1b[J: Clear remaining lines below content
    // \x1b[?2026l: End Synchronized Output (Commits frame atomically to screen)
    buffer.push_str("\x1b[J\x1b[?2026l");

    let mut out = stdout();
    out.write_all(buffer.as_bytes()).map_err(Error::Io)?;
    out.flush().map_err(Error::Io)?;
    Ok(())
}

// ── Box Drawing Functions matching User Fastfetch Specification ──────────────

fn render_box_header(title: &str, accent: (u8, u8, u8), width: usize) -> String {
    let title_len = title.len() + 2; // " TITLE "
    let remaining = if width > title_len + 2 { width - title_len - 2 } else { 2 };
    let left_dashes = remaining / 2;
    let right_dashes = remaining - left_dashes;

    let border_color = format!("\x1b[38;2;{};{};{}m", COLOR_BORDER.0, COLOR_BORDER.1, COLOR_BORDER.2);
    let accent_color = format!("\x1b[38;2;{};{};{}m", accent.0, accent.1, accent.2);
    let reset = "\x1b[0m";

    format!(
        " {border_color}┌{dashes_l}{accent_color} {title} {border_color}{dashes_r}┐{reset}",
        border_color = border_color,
        dashes_l = "─".repeat(left_dashes),
        accent_color = accent_color,
        title = title,
        dashes_r = "─".repeat(right_dashes),
        reset = reset
    )
}

fn render_box_item(
    is_first: bool,
    is_last: bool,
    _key: &str,
    content: &str,
    accent: (u8, u8, u8),
    width: usize,
    is_selected: bool,
) -> String {
    let accent_color = format!("\x1b[38;2;{};{};{}m", accent.0, accent.1, accent.2);
    let reset = "\x1b[0m";

    let prefix_char = if is_first {
        "┌─"
    } else if is_last {
        "└─"
    } else {
        "├─"
    };

    let visible_content_len = strip_ansi_len(content);
    let cursor_prefix = if is_selected {
        "❯ ".to_string()
    } else {
        "  ".to_string()
    };

    // Increase inner width by 1 to add an extra space before the right border
    let inner_width = if width > 5 { width - 5 } else { 20 };
    // selected row uses a 3-char accent block: leading space + '❯' + trailing space
    let cursor_visible_len = if is_selected { 3 } else { 2 };
    let content_len_with_cursor = visible_content_len + cursor_visible_len;
    let padding_len = if inner_width >= content_len_with_cursor {
        inner_width - content_len_with_cursor
    } else {
        0
    };

    let padding = " ".repeat(padding_len);

    if is_selected {
        // Only the arrow gets the accent background and white foreground;
        // the rest of the row stays transparent so the terminal background shows through.
        let arrow_bg = format!("\x1b[48;2;{};{};{}m", accent.0, accent.1, accent.2);
        let arrow_fg = "\x1b[1;38;2;255;255;255m";
        format!(
            " {accent_color}{prefix}{reset}{arrow_bg}{arrow_fg} {cursor}{reset}{content}{padding}  {accent_color}│{reset}",
            accent_color = accent_color,
            prefix = prefix_char,
            reset = reset,
            arrow_bg = arrow_bg,
            arrow_fg = arrow_fg,
            cursor = cursor_prefix,
            content = content,
            padding = padding
        )
    } else {
        format!(
            " {accent_color}{prefix}{reset} {cursor_prefix}{content}{padding} {accent_color}│{reset}",
            accent_color = accent_color,
            prefix = prefix_char,
            reset = reset,
            cursor_prefix = cursor_prefix,
            content = content,
            padding = padding
        )
    }
}

fn render_box_footer(width: usize) -> String {
    let border_color = format!("\x1b[38;2;{};{};{}m", COLOR_BORDER.0, COLOR_BORDER.1, COLOR_BORDER.2);
    let reset = "\x1b[0m";
    let inner_dashes = if width > 2 { width - 2 } else { 10 };

    format!(" {border_color}└{}┘{reset}", "─".repeat(inner_dashes))
}

fn render_palette_dots() -> String {
    // 8 aesthetic circles like Fastfetch
    let dots = [
        (82, 90, 107),   // Gray
        (255, 153, 153), // Pink
        (139, 212, 156), // Green
        (231, 199, 135), // Yellow
        (125, 166, 216), // Blue
        (215, 160, 201), // Purple
        (136, 192, 208), // Cyan
        (255, 255, 255), // White
    ];

    let mut s = String::from("   ");
    for (r, g, b) in dots {
        s.push_str(&format!("\x1b[38;2;{};{};{}m● \x1b[0m", r, g, b));
    }
    s
}

fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            // Skip zero-width unicode format characters and variation selectors
            if c == '\u{FE0F}' || c == '\u{FE0E}' || c == '\u{200D}' {
                continue;
            }
            if !c.is_control() {
                if (c >= '\u{1F300}' && c <= '\u{1FAFF}') || (c >= '\u{2600}' && c <= '\u{27BF}') {
                    len += 2;
                } else {
                    len += 1;
                }
            }
        }
    }
    len
}

// ── Interactive Sub-Prompts ─────────────────────────────────────────────────

fn prompt_custom_color() -> Result<Option<Color>> {
    let mut input = String::new();

    loop {
        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push(render_box_header("CUSTOM COLOR INPUT", COLOR_PRESETS, CARD_WIDTH));

        let prompt_desc = "Enter HEX (#FF9999 / FF9999), RGB (255 153 153), or name:";
        lines.push(render_box_item(true, false, "INFO", prompt_desc, COLOR_PRESETS, CARD_WIDTH, false));

        let current_input_line = format!("Input: \x1b[1;37m{}\x1b[0m█", input);
        lines.push(render_box_item(false, false, "INPUT", &current_input_line, COLOR_PRESETS, CARD_WIDTH, false));

        // Real-time preview calculation
        let parsed_color = parse_custom_input(&input);
        let preview_line = match parsed_color {
            Ok(c) => format!(
                "Live Preview: {}████████\x1b[0m  {} ({:3}, {:3}, {:3}) \x1b[38;2;139;212;156m[VALID]\x1b[0m",
                c.ansi_fg(), c.to_hex(), c.r, c.g, c.b
            ),
            Err(_) if input.is_empty() => "Live Preview: (type a hex code or RGB values)".into(),
            Err(_) => "\x1b[38;2;255;100;100m[ERR] Invalid format (e.g. #FF5500 or 255 85 0)\x1b[0m".into(),
        };
        lines.push(render_box_item(false, true, "PREVIEW", &preview_line, COLOR_PRESETS, CARD_WIDTH, false));
        lines.push(render_box_footer(CARD_WIDTH));

        lines.push(String::new());
        lines.push("  \x1b[38;2;122;132;153m[Enter] Confirm  •  [Esc] Cancel  •  [Backspace] Delete\x1b[0m".into());

        flush_lines_to_terminal(&lines)?;

        if let Event::Key(key) = read().map_err(Error::Io)? {
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => {
                    if let Ok(c) = parse_custom_input(&input) {
                        return Ok(Some(c));
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    if input.len() < 30 {
                        input.push(c);
                    }
                }
                _ => {}
            }
        }
    }
}

fn parse_custom_input(input: &str) -> Result<Color> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidColorValue("empty input".into()));
    }

    // Try hex
    if let Ok(color) = Color::from_hex(trimmed) {
        return Ok(color);
    }

    // Try preset named colors
    let named_presets = [
        ("pink", Color::new(255, 153, 153)),
        ("green", Color::new(172, 217, 119)),
        ("blue", Color::new(114, 176, 246)),
        ("violet", Color::new(208, 153, 255)),
        ("orange", Color::new(236, 187, 110)),
        ("nord", Color::new(136, 192, 208)),
        ("cyan", Color::new(136, 192, 208)),
        ("purple", Color::new(208, 153, 255)),
        ("red", Color::new(255, 100, 100)),
        ("yellow", Color::new(249, 215, 47)),
        ("white", Color::new(255, 255, 255)),
        ("black", Color::new(0, 0, 0)),
    ];

    if let Some((_, c)) = named_presets.iter().find(|(name, _)| *name == trimmed.to_ascii_lowercase()) {
        return Ok(*c);
    }

    // Try RGB components
    let parts: Vec<&str> = trimmed
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|p| !p.is_empty())
        .collect();

    if parts.len() == 3 {
        let r = parts[0].parse::<u8>().map_err(|_| Error::InvalidColorValue(trimmed.into()))?;
        let g = parts[1].parse::<u8>().map_err(|_| Error::InvalidColorValue(trimmed.into()))?;
        let b = parts[2].parse::<u8>().map_err(|_| Error::InvalidColorValue(trimmed.into()))?;
        return Ok(Color::new(r, g, b));
    }

    Err(Error::InvalidColorValue(trimmed.into()))
}

fn prompt_backup_filename() -> Result<Option<PathBuf>> {
    let default_name = "syscolor-backup.json".to_string();
    let mut input = default_name;

    loop {
        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push(render_box_header("BACKUP REGISTRY COLORS", COLOR_ACTIONS, CARD_WIDTH));
        lines.push(render_box_item(
            true,
            false,
            "INFO",
            "Enter destination JSON file path for backup:",
            COLOR_ACTIONS,
            CARD_WIDTH,
            false,
        ));
        let inp_line = format!("File: \x1b[1;37m{}\x1b[0m█", input);
        lines.push(render_box_item(false, true, "PATH", &inp_line, COLOR_ACTIONS, CARD_WIDTH, false));
        lines.push(render_box_footer(CARD_WIDTH));

        lines.push(String::new());
        lines.push("  \x1b[38;2;122;132;153m[Enter] Save Backup  •  [Esc] Cancel\x1b[0m".into());

        flush_lines_to_terminal(&lines)?;

        if let Event::Key(key) = read().map_err(Error::Io)? {
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter => {
                    let trimmed = input.trim();
                    if !trimmed.is_empty() {
                        return Ok(Some(PathBuf::from(trimmed)));
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    if input.len() < 100 {
                        input.push(c);
                    }
                }
                _ => {}
            }
        }
    }
}

fn prompt_restore_file(manager: &SystemColorManager) -> Result<Option<PathBuf>> {
    let backup_files = find_json_files()?;
    let mut selected_file_idx = 0;

    if backup_files.is_empty() {
        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push(render_box_header("RESTORE BACKUP", COLOR_ACTIONS, CARD_WIDTH));
        lines.push(render_box_item(
            true,
            true,
            "INFO",
            "No *.json backup files found in the current directory.",
            COLOR_ACTIONS,
            CARD_WIDTH,
            false,
        ));
        lines.push(render_box_footer(CARD_WIDTH));
        lines.push(String::new());
        lines.push("  \x1b[38;2;122;132;153mPress any key to return to main menu...\x1b[0m".into());

        flush_lines_to_terminal(&lines)?;
        let _ = read();
        return Ok(None);
    }

    loop {
        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push(render_box_header("SELECT BACKUP FILE TO RESTORE", COLOR_ACTIONS, CARD_WIDTH));

        for (i, file) in backup_files.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == backup_files.len() - 1;
            let is_selected = selected_file_idx == i;

            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);
            let size_kb = (size as f64) / 1024.0;

            let file_label = format!("{:<32} ({:.1} KB)", name, size_kb);

            lines.push(render_box_item(
                is_first,
                is_last,
                &format!("F{}", i + 1),
                &file_label,
                COLOR_ACTIONS,
                CARD_WIDTH,
                is_selected,
            ));
        }
        lines.push(render_box_footer(CARD_WIDTH));

        lines.push(String::new());
        lines.push("  \x1b[38;2;122;132;153m[↑/↓] Select File  •  [Enter] Restore & Apply  •  [Esc] Cancel\x1b[0m".into());

        flush_lines_to_terminal(&lines)?;

        if let Event::Key(key) = read().map_err(Error::Io)? {
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected_file_idx == 0 {
                        selected_file_idx = backup_files.len() - 1;
                    } else {
                        selected_file_idx -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    selected_file_idx = (selected_file_idx + 1) % backup_files.len();
                }
                KeyCode::Enter => {
                    let target_path = &backup_files[selected_file_idx];
                    restore_registry_from_file(target_path)?;
                    manager.apply()?;
                    return Ok(Some(target_path.clone()));
                }
                _ => {}
            }
        }
    }
}

fn show_registry_dump_screen() -> Result<()> {
    let entries = registry::read_all_entries()?;

    let mut scroll_offset = 0;
    let max_visible = 18;

    let entries_vec: Vec<(String, Color)> = entries.into_iter().collect();

    loop {
        let mut lines = Vec::new();
        lines.push(String::new());
        lines.push(render_box_header("REGISTRY DUMP (HKCU\\Control Panel\\Colors)", COLOR_STATUS, CARD_WIDTH));

        let visible_entries = entries_vec.iter().skip(scroll_offset).take(max_visible);
        let total = entries_vec.len();

        for (i, (key, color)) in visible_entries.enumerate() {
            let is_first = i == 0;
            let is_last = i == max_visible - 1 || scroll_offset + i == total - 1;

            let row = format!(
                "{:<24} {} ({:3}, {:3}, {:3})  {}████\x1b[0m",
                key, color.to_hex(), color.r, color.g, color.b, color.ansi_fg()
            );

            lines.push(render_box_item(
                is_first,
                is_last,
                "REG",
                &row,
                COLOR_STATUS,
                CARD_WIDTH,
                false,
            ));
        }
        lines.push(render_box_footer(CARD_WIDTH));

        lines.push(String::new());
        let page_info = format!(
            "  \x1b[38;2;122;132;153mShowing {}-{} of {} entries • [↑/↓] Scroll • [Esc/q/Enter] Return\x1b[0m",
            scroll_offset + 1,
            (scroll_offset + max_visible).min(total),
            total
        );
        lines.push(page_info);

        flush_lines_to_terminal(&lines)?;

        if let Event::Key(key) = read().map_err(Error::Io)? {
            if key.kind == crossterm::event::KeyEventKind::Release {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => break,
                KeyCode::Up | KeyCode::Char('k') => {
                    if scroll_offset > 0 {
                        scroll_offset -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if scroll_offset + max_visible < total {
                        scroll_offset += 1;
                    }
                }
                KeyCode::PageUp => {
                    scroll_offset = scroll_offset.saturating_sub(max_visible);
                }
                KeyCode::PageDown => {
                    if scroll_offset + max_visible < total {
                        scroll_offset = (scroll_offset + max_visible).min(total - 1);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn find_json_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn backup_registry_to_file(path: &Path) -> Result<()> {
    #[derive(serde::Serialize)]
    struct Backup {
        colors: BTreeMap<String, String>,
    }

    let entries = registry::read_all_entries()?;
    let colors = entries
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();

    let json = serde_json::to_string_pretty(&Backup { colors })
        .map_err(|e| Error::Win32(e.to_string()))?;
    fs::write(path, json).map_err(Error::Io)?;
    Ok(())
}

fn restore_registry_from_file(path: &Path) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Backup {
        colors: BTreeMap<String, String>,
    }

    let content = fs::read_to_string(path).map_err(Error::Io)?;
    let backup: Backup = serde_json::from_str(&content)
        .map_err(|e| Error::Win32(e.to_string()))?;

    let mut entries = BTreeMap::new();
    for (key, val) in backup.colors {
        if let Ok(color) = Color::from_registry(&val) {
            entries.insert(key, color);
        }
    }

    registry::write_all_entries(&entries)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_mode_cycles_properly() {
        let mode = TargetMode::Both;
        assert_eq!(mode.next(), TargetMode::Highlight);
        assert_eq!(mode.next().next(), TargetMode::HotTracking);
        assert_eq!(mode.next().next().next(), TargetMode::Both);
    }

    #[test]
    fn parses_custom_input_hex_and_rgb() {
        let hex_color = parse_custom_input("#FF9999").expect("valid hex");
        assert_eq!(hex_color, Color::new(255, 153, 153));

        let hex_no_hash = parse_custom_input("ACD977").expect("valid hex without hash");
        assert_eq!(hex_no_hash, Color::new(172, 217, 119));

        let rgb_space = parse_custom_input("114 176 246").expect("valid rgb");
        assert_eq!(rgb_space, Color::new(114, 176, 246));

        let rgb_comma = parse_custom_input("208, 153, 255").expect("valid rgb with commas");
        assert_eq!(rgb_comma, Color::new(208, 153, 255));

        let named = parse_custom_input("nord").expect("named preset");
        assert_eq!(named, Color::new(136, 192, 208));
    }

    #[test]
    fn strips_ansi_and_calculates_visible_width() {
        let plain = "Hello World";
        assert_eq!(strip_ansi_len(plain), 11);

        let colored = "\x1b[38;2;139;212;156mHello World\x1b[0m";
        assert_eq!(strip_ansi_len(colored), 11);

        let option_text = "[1] Pink         #FF9999 (255, 153, 153)";
        assert_eq!(strip_ansi_len(option_text), 40);
    }
}
