#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{COLOR_HIGHLIGHT, COLOR_HOTLIGHT, SYS_COLOR_INDEX};

pub const REGISTRY_COLORS_PATH: &str = r"Control Panel\Colors";
pub const REGISTRY_HILIGHT: &str = "Hilight";
pub const REGISTRY_HOT_TRACKING_COLOR: &str = "HotTrackingColor";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemColorSlot {
    Highlight,
    HotTracking,
}

impl SystemColorSlot {
    pub const fn registry_name(self) -> &'static str {
        match self {
            Self::Highlight => REGISTRY_HILIGHT,
            Self::HotTracking => REGISTRY_HOT_TRACKING_COLOR,
        }
    }

    pub const fn as_sys_color_index(self) -> SYS_COLOR_INDEX {
        match self {
            Self::Highlight => COLOR_HIGHLIGHT,
            Self::HotTracking => COLOR_HOTLIGHT,
        }
    }
}