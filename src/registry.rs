use crate::{
    color::Color,
    constants::{SystemColorSlot, REGISTRY_COLORS_PATH},
    errors::Result,
};

use std::collections::BTreeMap;

#[cfg(windows)]
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

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

pub fn read_all_entries() -> Result<BTreeMap<String, Color>> {
    #[cfg(windows)]
    {
        let key = open_colors_key()?;
        let mut entries = BTreeMap::new();

        for value_result in key.enum_values() {
            let (name, value) = value_result?;
            if let Ok(color) = Color::from_registry(&value.bytes_to_string()) {
                entries.insert(name, color);
            }
        }

        return Ok(entries);
    }

    #[cfg(not(windows))]
    {
        Ok(BTreeMap::new())
    }
}

pub fn write_all_entries(entries: &BTreeMap<String, Color>) -> Result<()> {
    #[cfg(windows)]
    {
        let key = open_colors_key()?;

        for (name, color) in entries {
            key.set_value(name, &color.to_string())?;
        }

        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let _ = entries;
        Err(crate::errors::Error::UnsupportedPlatform(
            "registry access requires Windows",
        ))
    }
}

pub fn read_all() -> Result<crate::system_colors::SystemColors> {
    Ok(crate::system_colors::SystemColors {
        highlight: read_color(SystemColorSlot::Highlight)?,
        hot_tracking: read_color(SystemColorSlot::HotTracking)?,
    })
}

#[cfg(windows)]
trait RegistryBytesExt {
    fn bytes_to_string(&self) -> String;
}

#[cfg(windows)]
impl RegistryBytesExt for winreg::RegValue {
    fn bytes_to_string(&self) -> String {
        String::from_utf8_lossy(&self.bytes)
            .trim_end_matches('\u{0}')
            .trim()
            .to_owned()
    }
}

#[cfg(windows)]
fn open_colors_key() -> Result<RegKey> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    Ok(root.open_subkey_with_flags(REGISTRY_COLORS_PATH, KEY_READ | KEY_WRITE)?)
}

#[cfg(test)]
mod tests {
    use super::open_colors_key;

    #[test]
    #[cfg(windows)]
    fn opens_colors_key_with_write_access() {
        let _ = open_colors_key().expect("colors registry key should open");
    }
}