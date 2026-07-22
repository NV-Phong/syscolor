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

    #[cfg(windows)]
    #[test]
    fn converts_to_and_from_colorref() {
        let color = Color::new(172, 217, 119);

        assert_eq!(Color::from_colorref(color.to_colorref()), color);
    }
}