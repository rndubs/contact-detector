//! Contact Detector CLI Application

use clap::Parser;
use contact_detector::Result;

#[cfg(feature = "exodus")]
use contact_detector::io::ExodusReader;

mod cli;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    let log_level = if cli.debug {
        "debug"
    } else if cli.verbose {
        "info"
    } else {
        "warn"
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Dispatch to command handlers
    match cli.command {
        Commands::Info { input } => cmd_info(input),
        Commands::Skin {
            input,
            output,
            part,
        } => cmd_skin(input, output, part),
        Commands::Contact {
            input,
            part_a,
            part_b,
            max_gap,
            max_penetration,
            max_angle,
            output,
        } => cmd_contact(
            input,
            part_a,
            part_b,
            max_gap,
            max_penetration,
            max_angle,
            output,
        ),
        Commands::Analyze {
            input,
            pairs,
            config,
            output,
        } => cmd_analyze(input, pairs, config, output),
    }
}

fn cmd_info(input: std::path::PathBuf) -> Result<()> {
    println!("Reading mesh file: {}", input.display());

    // Try to read as JSON first, then Exodus if available
    let mesh = if input.extension().and_then(|s| s.to_str()) == Some("json") {
        contact_detector::io::read_json_mesh(&input)?
    } else {
        #[cfg(feature = "exodus")]
        {
            let reader = ExodusReader::open(&input)?;
            reader.read_mesh()?
        }
        #[cfg(not(feature = "exodus"))]
        {
            return Err(contact_detector::ContactDetectorError::ConfigError(
                "Exodus support not compiled in. Install libhdf5-dev and libnetcdf-dev, then rebuild with --features exodus".to_string()
            ));
        }
    };

    println!("\n{}", "=".repeat(60));
    println!("MESH INFORMATION");
    println!("{}", "=".repeat(60));
    println!();
    println!("  Nodes:        {}", mesh.num_nodes());
    println!("  Elements:     {}", mesh.num_elements());
    println!("  Blocks:       {}", mesh.num_blocks());
    println!("  Node Sets:    {}", mesh.node_sets.len());
    println!("  Side Sets:    {}", mesh.side_sets.len());
    println!();

    if !mesh.element_blocks.is_empty() {
        println!("Element Blocks:");
        let mut blocks: Vec<_> = mesh.element_blocks.iter().collect();
        blocks.sort_by_key(|(name, _)| *name);
        for (name, elements) in blocks {
            println!("  - {}: {} elements", name, elements.len());
        }
        println!();
    }

    if !mesh.node_sets.is_empty() {
        println!("Node Sets:");
        let mut nodesets: Vec<_> = mesh.node_sets.iter().collect();
        nodesets.sort_by_key(|(name, _)| *name);
        for (name, nodes) in nodesets {
            println!("  - {}: {} nodes", name, nodes.len());
        }
        println!();
    }

    if !mesh.side_sets.is_empty() {
        println!("Side Sets:");
        let mut sidesets: Vec<_> = mesh.side_sets.iter().collect();
        sidesets.sort_by_key(|(name, _)| *name);
        for (name, sides) in sidesets {
            println!("  - {}: {} sides", name, sides.len());
        }
        println!();
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

fn cmd_skin(
    _input: std::path::PathBuf,
    _output: std::path::PathBuf,
    _part: Option<String>,
) -> Result<()> {
    println!("Surface extraction not yet implemented (Phase 2)");
    Ok(())
}

fn cmd_contact(
    _input: std::path::PathBuf,
    _part_a: String,
    _part_b: String,
    _max_gap: f64,
    _max_penetration: f64,
    _max_angle: f64,
    _output: std::path::PathBuf,
) -> Result<()> {
    println!("Contact detection not yet implemented (Phase 3)");
    Ok(())
}

fn cmd_analyze(
    _input: std::path::PathBuf,
    _pairs: String,
    _config: Option<std::path::PathBuf>,
    _output: std::path::PathBuf,
) -> Result<()> {
    println!("Full analysis not yet implemented (Phase 4)");
    Ok(())
}
