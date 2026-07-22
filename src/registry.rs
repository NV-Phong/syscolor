use crate::{
    color::Color,
    constants::{SystemColorSlot, REGISTRY_COLORS_PATH},
    errors::Result,
};

#[cfg(windows)]
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

pub fn read_color(slot: SystemColorSlot) -> Result<Color> {
    #[cfg(windows)]
    {
        let key = open_colors_key()?;
        let value: String = key.get_value(slot.registry_name())?;
        return Color::from_registry(&value);
    }

    #[cfg(not(windows))]
    {
        let _ = slot;
        Err(crate::errors::Error::UnsupportedPlatform("registry access requires Windows"))
    }
}

pub fn write_color(slot: SystemColorSlot, color: Color) -> Result<()> {
    #[cfg(windows)]
    {
        let key = open_colors_key()?;
        key.set_value(slot.registry_name(), &color.to_string())?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let _ = (slot, color);
        Err(crate::errors::Error::UnsupportedPlatform("registry access requires Windows"))
    }
}

pub fn read_all() -> Result<crate::system_colors::SystemColors> {
    Ok(crate::system_colors::SystemColors {
        highlight: read_color(SystemColorSlot::Highlight)?,
        hot_tracking: read_color(SystemColorSlot::HotTracking)?,
    })
}

#[cfg(windows)]
fn open_colors_key() -> Result<RegKey> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    Ok(root.open_subkey(REGISTRY_COLORS_PATH)?)
}