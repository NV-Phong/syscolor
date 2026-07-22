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
        target: ColorTarget,
        red: u8,
        green: u8,
        blue: u8,
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
        Commands::Set {
            target,
            red,
            green,
            blue,
        } => {
            tracing::info!(target = target.as_ref(), red, green, blue, "updating registry color");
            manager.set(target.slot(), Color::new(red, green, blue))?;
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