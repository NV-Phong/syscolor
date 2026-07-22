use crate::{
    color::Color,
    constants::SystemColorSlot,
    errors::{Error, Result},
};

#[cfg(windows)]
use windows::Win32::{
    Foundation::{COLORREF, LPARAM, WPARAM},
    Graphics::Gdi::{GetSysColor, SetSysColors},
    UI::WindowsAndMessaging::{
        HWND_BROADCAST, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        WM_SYSCOLORCHANGE,
    },
};

#[cfg(windows)]
pub fn get_sys_color(slot: SystemColorSlot) -> Result<Color> {
    let raw_color = unsafe { GetSysColor(slot.as_sys_color_index()) };
    Ok(Color::from_colorref(COLORREF(raw_color)))
}

#[cfg(not(windows))]
pub fn get_sys_color(_slot: SystemColorSlot) -> Result<Color> {
    Err(Error::UnsupportedPlatform("system color queries require Windows"))
}

#[cfg(windows)]
pub fn set_sys_colors(updates: &[(SystemColorSlot, Color)]) -> Result<()> {
    let indices: Vec<i32> = updates.iter().map(|(slot, _)| slot.as_sys_color_index().0).collect();
    let colors: Vec<COLORREF> = updates.iter().map(|(_, color)| color.to_colorref()).collect();

    unsafe { SetSysColors(indices.len() as i32, indices.as_ptr(), colors.as_ptr()) }
        .map_err(|error| Error::Win32(error.to_string()))
}

#[cfg(not(windows))]
pub fn set_sys_colors(_updates: &[(SystemColorSlot, Color)]) -> Result<()> {
    Err(Error::UnsupportedPlatform("system color updates require Windows"))
}

#[cfg(windows)]
pub fn broadcast_message(message: u32) -> Result<()> {
    let outcome = unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            message,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            1_000,
            None,
        )
    };

    if outcome.0 != 0 {
        Ok(())
    } else {
        Err(Error::Win32(format!("SendMessageTimeoutW failed for message {message}")))
    }
}

#[cfg(not(windows))]
pub fn broadcast_message(_message: u32) -> Result<()> {
    Err(Error::UnsupportedPlatform("broadcasting requires Windows"))
}

#[cfg(windows)]
pub const WM_SYSCOLORCHANGE_MESSAGE: u32 = WM_SYSCOLORCHANGE;

#[cfg(windows)]
pub const WM_SETTINGCHANGE_MESSAGE: u32 = WM_SETTINGCHANGE;