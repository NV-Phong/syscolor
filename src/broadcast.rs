use crate::{errors::Result, win32};

pub fn broadcast_system_color_change() -> Result<()> {
    win32::broadcast_message(win32::WM_SYSCOLORCHANGE_MESSAGE)
}

pub fn broadcast_setting_change() -> Result<()> {
    win32::broadcast_message(win32::WM_SETTINGCHANGE_MESSAGE)
}

pub fn broadcast_all() -> Result<()> {
    broadcast_system_color_change()?;
    broadcast_setting_change()?;
    Ok(())
}