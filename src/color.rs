use std::fmt;

use crate::errors::{Error, Result};

#[cfg(windows)]
use windows::Win32::Foundation::COLORREF;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_registry(value: &str) -> Result<Self> {
        let components: Vec<&str> = value
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .filter(|part| !part.is_empty())
            .collect();

        if components.len() != 3 {
            return Err(Error::InvalidColorValue(value.to_owned()));
        }

        let red = parse_component(components[0], value)?;
        let green = parse_component(components[1], value)?;
        let blue = parse_component(components[2], value)?;

        Ok(Self::new(red, green, blue))
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        let cleaned = hex.trim().trim_start_matches('#');
        if cleaned.len() == 6 {
            let r = u8::from_str_radix(&cleaned[0..2], 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            let g = u8::from_str_radix(&cleaned[2..4], 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            let b = u8::from_str_radix(&cleaned[4..6], 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            Ok(Self::new(r, g, b))
        } else if cleaned.len() == 3 {
            let r_str = format!("{}{}", &cleaned[0..1], &cleaned[0..1]);
            let g_str = format!("{}{}", &cleaned[1..2], &cleaned[1..2]);
            let b_str = format!("{}{}", &cleaned[2..3], &cleaned[2..3]);
            let r = u8::from_str_radix(&r_str, 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            let g = u8::from_str_radix(&g_str, 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            let b = u8::from_str_radix(&b_str, 16)
                .map_err(|_| Error::InvalidColorValue(hex.to_owned()))?;
            Ok(Self::new(r, g, b))
        } else {
            Err(Error::InvalidColorValue(hex.to_owned()))
        }
    }

    pub fn ansi_fg(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    #[allow(dead_code)]
    pub fn ansi_bg(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }

    #[cfg(windows)]
    pub const fn to_colorref(self) -> COLORREF {
        COLORREF((self.r as u32) | ((self.g as u32) << 8) | ((self.b as u32) << 16))
    }

    #[cfg(windows)]
    pub const fn from_colorref(color: COLORREF) -> Self {
        let raw = color.0;
        Self {
            r: (raw & 0xFF) as u8,
            g: ((raw >> 8) & 0xFF) as u8,
            b: ((raw >> 16) & 0xFF) as u8,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {} {}", self.r, self.g, self.b)
    }
}

impl fmt::Debug for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Color")
            .field("r", &self.r)
            .field("g", &self.g)
            .field("b", &self.b)
            .finish()
    }
}

fn parse_component(component: &str, original: &str) -> Result<u8> {
    component
        .parse::<u8>()
        .map_err(|_| Error::InvalidColorValue(original.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn parses_registry_color() {
        let color = Color::from_registry("172 217 119").expect("valid color");

        assert_eq!(color, Color::new(172, 217, 119));
    }

    #[test]
    fn display_uses_registry_format() {
        let color = Color::new(1, 2, 3);

        assert_eq!(color.to_string(), "1 2 3");
    }

    #[test]
    fn parses_and_formats_hex() {
        let color = Color::from_hex("#FF9999").expect("valid hex");
        assert_eq!(color, Color::new(255, 153, 153));
        assert_eq!(color.to_hex(), "#FF9999");

        let short_color = Color::from_hex("f99").expect("valid short hex");
        assert_eq!(short_color, Color::new(255, 153, 153));
    }

    #[cfg(windows)]
    #[test]
    fn converts_to_and_from_colorref() {
        let color = Color::new(172, 217, 119);

        assert_eq!(Color::from_colorref(color.to_colorref()), color);
    }
}