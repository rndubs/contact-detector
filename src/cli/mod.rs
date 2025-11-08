//! CLI commands and interface
//!
//! This module defines the command-line interface structure using clap.
//! It provides commands for mesh inspection, surface extraction, and contact detection.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Command-line interface for the contact detector application
///
/// Provides commands for mesh inspection, surface extraction, and contact pair detection
/// for hexahedral finite element meshes.
///
/// # Example
/// ```bash
/// # Display mesh information
/// contact-detector info mesh.exo
///
/// # Extract surface
/// contact-detector skin mesh.exo -o surface.vtu
///
/// # Detect contact pairs
/// contact-detector contact mesh.exo --part-a Block1 --part-b Block2 -o result.vtu
/// ```
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

/// Available subcommands for the contact detector CLI
///
/// Each command provides specific functionality for working with hexahedral meshes
/// and detecting contact pairs between surfaces.
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
