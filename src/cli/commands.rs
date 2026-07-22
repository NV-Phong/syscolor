use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::{
    color::Color,
    constants::SystemColorSlot,
    errors::{Error, Result},
    system_colors::{SystemColorManager, SystemColors},
};

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
            tracing::warn!(?path, "backup is scaffolded but not implemented yet");
            return Err(Error::UnsupportedPlatform(
                "backup/restore will be added in the next iteration",
            ));
        }
        Commands::Restore { path } => {
            tracing::warn!(?path, "restore is scaffolded but not implemented yet");
            return Err(Error::UnsupportedPlatform(
                "backup/restore will be added in the next iteration",
            ));
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