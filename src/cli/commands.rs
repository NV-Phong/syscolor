use std::path::PathBuf;
use std::{collections::BTreeMap, fs};

use clap::{Parser, Subcommand, ValueEnum};

use crate::{
    color::Color,
    constants::SystemColorSlot,
    errors::{Error, Result},
    registry,
    system_colors::{SystemColorManager, SystemColors},
};

use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "syscolor", version, about = "Refresh and manage Windows system colors")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Apply,
    Reload,
    Get {
        target: ColorTarget,
    },
    Set {
        #[arg(trailing_var_arg = true, value_name = "ARGS")]
        args: Vec<String>,
    },
    Dump,
    Backup {
        path: PathBuf,
    },
    Restore {
        path: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorTarget {
    #[value(alias = "hilight")]
    Highlight,
    #[value(alias = "hottracking", alias = "hottrackingcolor", alias = "hot-light")]
    HotTracking,
}

impl ColorTarget {
    pub const fn slot(self) -> SystemColorSlot {
        match self {
            Self::Highlight => SystemColorSlot::Highlight,
            Self::HotTracking => SystemColorSlot::HotTracking,
        }
    }
}

pub fn run(cli: Cli) -> Result<()> {
    let manager = SystemColorManager;
    let command = cli.command.unwrap_or(Commands::Apply);

    match command {
        Commands::Apply => {
            tracing::info!("applying registry colors");
            manager.apply()?;
            tracing::info!("apply completed");
        }
        Commands::Reload => {
            tracing::info!("reloading system colors");
            manager.reload()?;
            tracing::info!("reload completed");
        }
        Commands::Get { target } => {
            let color = manager.get()?.color_for(target);
            println!("{} = {}", target.as_ref(), color);
        }
        Commands::Set { args } => {
            let (target, color) = parse_set_arguments(&args)?;
            if let Some(target) = target {
                tracing::info!(target = target.as_ref(), red = color.r, green = color.g, blue = color.b, "updating registry color");
                manager.set(target.slot(), color)?;
            } else {
                tracing::info!(red = color.r, green = color.g, blue = color.b, "updating both registry colors");
                manager.set_all(color)?;
            }
            tracing::info!("set completed");
        }
        Commands::Dump => {
            let colors = manager.dump_registry()?;
            print_colors("registry", colors);
        }
        Commands::Backup { path } => {
            tracing::info!(?path, "backing up registry colors");
            let backup = RegistryColorsBackup::from_registry(registry::read_all_entries()?);
            let json = serde_json::to_string_pretty(&backup).map_err(|error| Error::Win32(error.to_string()))?;
            fs::write(&path, json).map_err(|error| Error::Io(error))?;
            tracing::info!(?path, "backup completed");
        }
        Commands::Restore { path } => {
            tracing::info!(?path, "restoring registry colors");
            let json = fs::read_to_string(&path).map_err(|error| Error::Io(error))?;
            let backup: RegistryColorsBackup = serde_json::from_str(&json)
                .map_err(|error| Error::Win32(error.to_string()))?;
            registry::write_all_entries(&backup.into_registry_entries())?;
            manager.apply()?;
            tracing::info!(?path, "restore completed");
        }
    }

    Ok(())
}

impl SystemColors {
    pub fn color_for(self, target: ColorTarget) -> Color {
        match target {
            ColorTarget::Highlight => self.highlight,
            ColorTarget::HotTracking => self.hot_tracking,
        }
    }
}

impl AsRef<str> for ColorTarget {
    fn as_ref(&self) -> &str {
        match self {
            Self::Highlight => "highlight",
            Self::HotTracking => "hot-tracking",
        }
    }
}

fn print_colors(label: &str, colors: SystemColors) {
    println!("{}:", label);
    println!("  highlight = {}", colors.highlight);
    println!("  hot-tracking = {}", colors.hot_tracking);
}

fn parse_set_arguments(args: &[String]) -> Result<(Option<ColorTarget>, Color)> {
    let mut target = None;
    let mut remaining = args;

    if let Some(first) = args.first() {
        if let Some(parsed_target) = parse_target(first) {
            target = Some(parsed_target);
            remaining = &args[1..];
        }
    }

    match remaining {
        [value] => Ok((target, resolve_named_or_rgb_color(value)?)),
        [red, green, blue] => Ok((
            target,
            Color::new(
                parse_color_component(red)?,
                parse_color_component(green)?,
                parse_color_component(blue)?,
            ),
        )),
        _ => Err(Error::InvalidColorValue("expected a color name or RGB values".to_string())),
    }
}

fn parse_target(token: &str) -> Option<ColorTarget> {
    match token.to_ascii_lowercase().as_str() {
        "highlight" | "hilight" => Some(ColorTarget::Highlight),
        "hottracking" | "hot-tracking" | "hottrackingcolor" | "hot-light" => Some(ColorTarget::HotTracking),
        _ => None,
    }
}

fn resolve_named_or_rgb_color(token: &str) -> Result<Color> {
    if let Some(color) = preset_colors().iter().find(|(name, _)| *name == token.to_ascii_lowercase()) {
        return Ok(color.1);
    }

    let components: Vec<&str> = token
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .collect();

    if components.len() != 3 {
        return Err(Error::InvalidColorValue(token.to_owned()));
    }

    let red = parse_color_component(components[0])?;
    let green = parse_color_component(components[1])?;
    let blue = parse_color_component(components[2])?;

    Ok(Color::new(red, green, blue))
}

fn parse_color_component(component: &str) -> Result<u8> {
    component
        .trim()
        .parse::<u8>()
        .map_err(|_| Error::InvalidColorValue(component.to_owned()))
}

fn preset_colors() -> Vec<(&'static str, Color)> {
    vec![
        ("pink", Color::new(255, 153, 153)),
        ("green", Color::new(172, 217, 119)),
        ("blue", Color::new(114, 176, 246)),
        ("violet", Color::new(208, 153, 255)),
        ("orange", Color::new(236, 187, 110)),
    ]
}

#[derive(Debug, Serialize, Deserialize)]
struct RegistryColorsBackup {
    colors: BTreeMap<String, SerializableColor>,
}

impl RegistryColorsBackup {
    fn from_registry(entries: BTreeMap<String, Color>) -> Self {
        Self {
            colors: entries
                .into_iter()
                .map(|(name, color)| (name, SerializableColor::from(color)))
                .collect(),
        }
    }

    fn into_registry_entries(self) -> BTreeMap<String, Color> {
        self.colors
            .into_iter()
            .map(|(name, color)| (name, color.into()))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct SerializableColor {
    r: u8,
    g: u8,
    b: u8,
}

impl From<Color> for SerializableColor {
    fn from(color: Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

impl From<SerializableColor> for Color {
    fn from(color: SerializableColor) -> Self {
        Color::new(color.r, color.g, color.b)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_set_arguments, Color, ColorTarget};

    #[test]
    fn resolves_named_preset_colors() {
        let (target, color) = parse_set_arguments(&["violet".to_string()]).expect("preset should resolve");

        assert_eq!(target, None);
        assert_eq!(color, Color::new(208, 153, 255));
    }

    #[test]
    fn resolves_explicit_rgb_values() {
        let (target, color) = parse_set_arguments(&["172".to_string(), "217".to_string(), "119".to_string()]).expect("rgb values should resolve");

        assert_eq!(target, None);
        assert_eq!(color, Color::new(172, 217, 119));
    }

    #[test]
    fn resolves_with_explicit_target() {
        let (target, color) = parse_set_arguments(&["hot-tracking".to_string(), "orange".to_string()]).expect("target and color should resolve");

        assert_eq!(target, Some(ColorTarget::HotTracking));
        assert_eq!(color, Color::new(236, 187, 110));
    }
}