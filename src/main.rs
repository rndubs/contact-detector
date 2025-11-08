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
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    part: Option<String>,
) -> Result<()> {
    use contact_detector::io::{write_surface_to_vtu, write_surfaces_to_vtu};
    use contact_detector::mesh::extract_surface;

    log::info!("Reading mesh file: {}", input.display());

    // Read mesh from file
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

    log::info!(
        "Loaded mesh with {} nodes, {} elements",
        mesh.num_nodes(),
        mesh.num_elements()
    );

    // Extract surface
    let surfaces = extract_surface(&mesh)?;

    // Filter by part if specified
    let surfaces_to_write: Vec<_> = if let Some(part_name) = part {
        surfaces
            .into_iter()
            .filter(|s| s.part_name == part_name)
            .collect()
    } else {
        surfaces
    };

    if surfaces_to_write.is_empty() {
        log::warn!("No surfaces to write");
        return Ok(());
    }

    // Write output
    if surfaces_to_write.len() == 1 {
        // Single surface - write directly to output file
        write_surface_to_vtu(&surfaces_to_write[0], &output)?;
        println!("Surface extracted and written to: {}", output.display());
    } else {
        // Multiple surfaces - output should be a directory
        write_surfaces_to_vtu(&surfaces_to_write, &output)?;
        println!(
            "Extracted {} surfaces to directory: {}",
            surfaces_to_write.len(),
            output.display()
        );
    }

    // Print statistics
    for surface in &surfaces_to_write {
        println!(
            "  - {}: {} faces, total area: {:.6}",
            surface.part_name,
            surface.num_faces(),
            surface.total_area()
        );
    }

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
