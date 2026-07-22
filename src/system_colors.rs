use crate::{
    broadcast,
    color::Color,
    constants::SystemColorSlot,
    errors::Result,
    registry,
    win32,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemColors {
    pub highlight: Color,
    pub hot_tracking: Color,
}

#[derive(Default)]
pub struct SystemColorManager;

impl SystemColorManager {
    pub fn get(&self) -> Result<SystemColors> {
        Ok(SystemColors {
            highlight: win32::get_sys_color(SystemColorSlot::Highlight)?,
            hot_tracking: win32::get_sys_color(SystemColorSlot::HotTracking)?,
        })
    }

    pub fn apply(&self) -> Result<()> {
        let colors = registry::read_all()?;
        win32::set_sys_colors(&[
            (SystemColorSlot::Highlight, colors.highlight),
            (SystemColorSlot::HotTracking, colors.hot_tracking),
        ])?;
        broadcast::broadcast_all()?;
        Ok(())
    }

    pub fn reload(&self) -> Result<()> {
        self.apply()
    }

    pub fn set(&self, slot: SystemColorSlot, color: Color) -> Result<()> {
        registry::write_color(slot, color)?;
        self.apply()
    }

    pub fn dump_registry(&self) -> Result<SystemColors> {
        registry::read_all()
    }
}