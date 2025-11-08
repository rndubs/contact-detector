//! CLI commands and interface

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "contact-detector")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Subcommand to run
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Display information about an Exodus mesh file
    Info {
        /// Path to the Exodus II file
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },

    /// Extract surface mesh from hexahedral mesh
    Skin {
        /// Path to the Exodus II file
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output VTU file path
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,

        /// Part/block name to extract (if not specified, extracts all)
        #[arg(short, long)]
        part: Option<String>,
    },

    /// Detect contact pairs between surfaces
    Contact {
        /// Path to the Exodus II file
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// First part name
        #[arg(long)]
        part_a: String,

        /// Second part name
        #[arg(long)]
        part_b: String,

        /// Maximum gap distance (tolerance)
        #[arg(long, default_value = "0.005")]
        max_gap: f64,

        /// Maximum penetration distance
        #[arg(long, default_value = "0.001")]
        max_penetration: f64,

        /// Maximum normal angle in degrees
        #[arg(long, default_value = "45.0")]
        max_angle: f64,

        /// Output VTU file path
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
    },

    /// Full analysis pipeline
    Analyze {
        /// Path to the Exodus II file
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Contact pairs to detect (format: "Part1:Part2,Part3:Part4")
        #[arg(long)]
        pairs: String,

        /// Configuration file (JSON)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output directory
        #[arg(short, long, value_name = "DIR")]
        output: PathBuf,
    },
}
