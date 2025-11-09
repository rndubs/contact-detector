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

    // Parse VTK version if provided
    let vtk_version = if let Some(ref version_str) = cli.vtk_version {
        Some(cli::parse_vtk_version(version_str).map_err(|e| {
            contact_detector::ContactDetectorError::ConfigError(format!(
                "Failed to parse VTK version: {}",
                e
            ))
        })?)
    } else {
        None
    };

    // Dispatch to command handlers
    match cli.command {
        Commands::Info { input } => cmd_info(input),
        Commands::Skin {
            input,
            output,
            part,
        } => cmd_skin(input, output, part, vtk_version),
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
            vtk_version,
        ),
        Commands::Analyze {
            input,
            pairs,
            config,
            output,
        } => cmd_analyze(input, pairs, config, output, vtk_version),
        Commands::AutoContact {
            input,
            max_gap,
            max_penetration,
            max_angle,
            min_pairs,
            output,
            export_metadata,
            export_sidesets,
            visualize_with_skin,
        } => cmd_auto_contact(
            input,
            max_gap,
            max_penetration,
            max_angle,
            min_pairs,
            output,
            vtk_version,
            export_metadata,
            export_sidesets,
            visualize_with_skin,
        ),
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
    vtk_version: Option<(u8, u8)>,
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
        if let Some(surface) = surfaces_to_write.first() {
            write_surface_to_vtu(surface, &output, vtk_version)?;
            println!("Surface extracted and written to: {}", output.display());
        }
    } else {
        // Multiple surfaces - output should be a directory
        write_surfaces_to_vtu(&surfaces_to_write, &output, vtk_version)?;
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
    input: std::path::PathBuf,
    part_a: String,
    part_b: String,
    max_gap: f64,
    max_penetration: f64,
    max_angle: f64,
    output: std::path::PathBuf,
    vtk_version: Option<(u8, u8)>,
) -> Result<()> {
    use contact_detector::contact::{detect_contact_pairs, ContactCriteria};
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

    // Find the requested surfaces
    let surface_a = surfaces
        .iter()
        .find(|s| s.part_name == part_a)
        .ok_or_else(|| {
            contact_detector::ContactDetectorError::ElementBlockNotFound(part_a.clone())
        })?;

    let surface_b = surfaces
        .iter()
        .find(|s| s.part_name == part_b)
        .ok_or_else(|| {
            contact_detector::ContactDetectorError::ElementBlockNotFound(part_b.clone())
        })?;

    // Set up contact detection criteria
    let criteria = ContactCriteria::new(max_gap, max_penetration, max_angle);

    // Detect contact pairs
    let results = detect_contact_pairs(surface_a, surface_b, &criteria)?;

    // Print summary
    results.print_summary();

    // Compute surface metrics
    use contact_detector::contact::SurfaceMetrics;
    use contact_detector::io::write_surface_with_contact_metadata;

    let metrics_a = SurfaceMetrics::compute(&results, surface_a, true);
    let metrics_b = SurfaceMetrics::compute(&results, surface_b, false);

    metrics_a.print_summary(&surface_a.part_name);
    metrics_b.print_summary(&surface_b.part_name);

    // Write surface A with contact metadata
    write_surface_with_contact_metadata(surface_a, &results, &metrics_a, &output, vtk_version)?;

    println!(
        "\nWrote surface with contact metadata to: {}",
        output.display()
    );

    Ok(())
}

fn cmd_analyze(
    input: std::path::PathBuf,
    pairs: String,
    config_file: Option<std::path::PathBuf>,
    output: std::path::PathBuf,
    vtk_version: Option<(u8, u8)>,
) -> Result<()> {
    use contact_detector::config::AnalysisConfig;
    use contact_detector::contact::{detect_contact_pairs, SurfaceMetrics};
    use contact_detector::io::write_surface_with_contact_metadata;
    use contact_detector::mesh::extract_surface;
    use indicatif::{ProgressBar, ProgressStyle};

    log::info!("Starting batch analysis...");

    // Load or create configuration
    let config = if let Some(config_path) = config_file {
        AnalysisConfig::from_file(&config_path)?
    } else {
        use contact_detector::contact::ContactCriteria;
        AnalysisConfig::from_pairs_string(
            input.to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            &pairs,
            ContactCriteria::default(),
        )?
    };

    log::info!("Analyzing {} contact pairs", config.contact_pairs.len());

    // Read mesh
    println!("Reading mesh file: {}", config.input_file);
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
                "Exodus support not compiled in".to_string(),
            ));
        }
    };

    println!(
        "Loaded mesh: {} nodes, {} elements, {} blocks\n",
        mesh.num_nodes(),
        mesh.num_elements(),
        mesh.num_blocks()
    );

    // Extract surfaces
    println!("Extracting surfaces...");
    let surfaces = extract_surface(&mesh)?;
    println!("Extracted {} surfaces\n", surfaces.len());

    // Create output directory
    std::fs::create_dir_all(&output)?;

    // Setup progress bar
    let pb = ProgressBar::new(config.contact_pairs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Process each contact pair
    for (idx, pair_config) in config.contact_pairs.iter().enumerate() {
        pb.set_message(format!(
            "{} ↔ {}",
            pair_config.surface_a, pair_config.surface_b
        ));

        // Find surfaces
        let surface_a = surfaces
            .iter()
            .find(|s| s.part_name == pair_config.surface_a)
            .ok_or_else(|| {
                contact_detector::ContactDetectorError::ElementBlockNotFound(
                    pair_config.surface_a.clone(),
                )
            })?;

        let surface_b = surfaces
            .iter()
            .find(|s| s.part_name == pair_config.surface_b)
            .ok_or_else(|| {
                contact_detector::ContactDetectorError::ElementBlockNotFound(
                    pair_config.surface_b.clone(),
                )
            })?;

        // Detect contact pairs
        let results = detect_contact_pairs(surface_a, surface_b, &pair_config.criteria)?;

        // Compute metrics
        let metrics_a = SurfaceMetrics::compute(&results, surface_a, true);

        // Generate output filename
        let output_filename = pair_config.output_file.clone().unwrap_or_else(|| {
            format!(
                "contact_{}_{}.vtu",
                sanitize_filename(&pair_config.surface_a),
                sanitize_filename(&pair_config.surface_b)
            )
        });

        let output_path = output.join(&output_filename);

        // Write results
        write_surface_with_contact_metadata(surface_a, &results, &metrics_a, &output_path, vtk_version)?;

        // Print brief summary
        println!(
            "\n[{}/{}] {} ↔ {}:",
            idx + 1,
            config.contact_pairs.len(),
            pair_config.surface_a,
            pair_config.surface_b
        );
        println!(
            "  Pairs: {}, Unpaired: {}, Avg Distance: {:.6}",
            metrics_a.num_pairs, metrics_a.num_unpaired, metrics_a.avg_distance
        );
        println!("  Output: {}", output_filename);

        pb.inc(1);
    }

    pb.finish_with_message("Complete");

    println!("\n{}", "=".repeat(60));
    println!("BATCH ANALYSIS COMPLETE");
    println!("{}", "=".repeat(60));
    println!("Processed {} contact pairs", config.contact_pairs.len());
    println!("Results written to: {}", output.display());
    println!("{}", "=".repeat(60));

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn cmd_auto_contact(
    input: std::path::PathBuf,
    max_gap: f64,
    max_penetration: f64,
    max_angle: f64,
    min_pairs: usize,
    output: std::path::PathBuf,
    vtk_version: Option<(u8, u8)>,
    export_metadata: bool,
    export_sidesets: bool,
    visualize_with_skin: bool,
) -> Result<()> {
    use contact_detector::contact::{detect_contact_pairs, ContactCriteria, SurfaceMetrics};
    use contact_detector::io::{write_surface_with_contact_metadata, ContactMetadata};
    use contact_detector::mesh::extract_surface;
    use indicatif::{ProgressBar, ProgressStyle};

    #[cfg(feature = "exodus")]
    use contact_detector::io::{add_contact_sidesets_to_mesh, write_contact_surfaces_with_skin, write_exodus};

    println!("{}", "=".repeat(60));
    println!("AUTOMATIC CONTACT DETECTION");
    println!("{}", "=".repeat(60));
    println!();

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

    println!(
        "Loaded mesh: {} nodes, {} elements, {} blocks",
        mesh.num_nodes(),
        mesh.num_elements(),
        mesh.num_blocks()
    );
    println!();

    // Extract all surfaces
    println!("Extracting surfaces from all element blocks...");
    let surfaces = extract_surface(&mesh)?;
    println!("Extracted {} surfaces:", surfaces.len());
    for surface in &surfaces {
        println!(
            "  - {}: {} faces, area: {:.6}",
            surface.part_name,
            surface.num_faces(),
            surface.total_area()
        );
    }
    println!();

    // Set up contact detection criteria
    let criteria = ContactCriteria::new(max_gap, max_penetration, max_angle);

    println!("Contact detection criteria:");
    println!("  Max gap:         {:.6}", max_gap);
    println!("  Max penetration: {:.6}", max_penetration);
    println!("  Max angle:       {:.1}°", max_angle);
    println!("  Min pairs:       {}", min_pairs);
    println!();

    // Create output directory
    std::fs::create_dir_all(&output)?;

    // Test all pairs of surfaces
    let num_surfaces = surfaces.len();
    let total_tests = (num_surfaces * (num_surfaces - 1)) / 2; // n choose 2

    if total_tests == 0 {
        println!("Not enough surfaces to test for contact (need at least 2)");
        return Ok(());
    }

    println!("Testing {} surface pair combinations...", total_tests);
    println!("{}", "=".repeat(60));

    // Setup progress bar
    let pb = ProgressBar::new(total_tests as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let mut detected_pairs = Vec::new();

    // Test all unique pairs (i, j) where i < j
    for i in 0..num_surfaces {
        for j in (i + 1)..num_surfaces {
            let surface_a = &surfaces[i];
            let surface_b = &surfaces[j];

            pb.set_message(format!("{} ↔ {}", surface_a.part_name, surface_b.part_name));

            // Detect contact pairs
            let results = detect_contact_pairs(surface_a, surface_b, &criteria)?;

            // Check if this pair has significant contact
            if results.num_pairs() >= min_pairs {
                let metrics_a = SurfaceMetrics::compute(&results, surface_a, true);
                let metrics_b = SurfaceMetrics::compute(&results, surface_b, false);

                detected_pairs.push((
                    surface_a.part_name.clone(),
                    surface_b.part_name.clone(),
                    results,
                    metrics_a,
                    metrics_b,
                    i,
                    j,
                ));

                log::info!(
                    "Found contact: {} ↔ {} ({} pairs)",
                    surface_a.part_name,
                    surface_b.part_name,
                    detected_pairs.last().unwrap().2.num_pairs()
                );
            }

            pb.inc(1);
        }
    }

    pb.finish_with_message("Complete");
    println!();

    // Initialize metadata if export requested
    let mut metadata = if export_metadata {
        Some(ContactMetadata::new(
            input.to_string_lossy().to_string(),
            &criteria,
            min_pairs,
        ))
    } else {
        None
    };

    // Report results
    println!("{}", "=".repeat(60));
    println!("DETECTION RESULTS");
    println!("{}", "=".repeat(60));
    println!();

    if detected_pairs.is_empty() {
        println!("No contact pairs detected with the specified criteria.");
        println!();
        println!("Suggestions:");
        println!("  - Try increasing --max-gap (current: {:.6})", max_gap);
        println!("  - Try increasing --max-angle (current: {:.1}°)", max_angle);
        println!(
            "  - Try decreasing --min-pairs (current: {})",
            min_pairs
        );
    } else {
        println!(
            "Detected {} contact pair(s):",
            detected_pairs.len()
        );
        println!();

        // Write output files for each detected pair
        for (idx, (part_a, part_b, results, metrics_a, metrics_b, i, j)) in
            detected_pairs.iter().enumerate()
        {
            println!(
                "[{}/{}] {} ↔ {}:",
                idx + 1,
                detected_pairs.len(),
                part_a,
                part_b
            );
            println!("  Contact pairs:   {}", results.num_pairs());
            println!("  Unpaired (A):    {}", results.unpaired_a.len());
            println!("  Unpaired (B):    {}", results.unpaired_b.len());
            println!("  Avg distance:    {:.6}", metrics_a.avg_distance);
            println!("  Min distance:    {:.6}", metrics_a.min_distance);
            println!("  Max distance:    {:.6}", metrics_a.max_distance);

            // Add to metadata if export requested
            if let Some(ref mut meta) = metadata {
                meta.add_contact_pair(
                    idx + 1,
                    &surfaces[*i],
                    &surfaces[*j],
                    results,
                    metrics_a,
                    metrics_b,
                );
            }

            // Generate output filename
            let output_filename = format!(
                "contact_{}_{}.vtu",
                sanitize_filename(part_a),
                sanitize_filename(part_b)
            );

            let output_path = output.join(&output_filename);

            // Write results - use enhanced visualization if requested
            if visualize_with_skin {
                #[cfg(feature = "exodus")]
                {
                    write_contact_surfaces_with_skin(
                        &surfaces[*i],
                        &surfaces[*j],
                        results,
                        &surfaces,
                        part_a,
                        part_b,
                        idx + 1,
                        &output_path,
                        vtk_version,
                    )?;
                }
                #[cfg(not(feature = "exodus"))]
                {
                    log::warn!("--visualize-with-skin requires exodus feature, falling back to standard output");
                    write_surface_with_contact_metadata(
                        &surfaces[*i],
                        results,
                        metrics_a,
                        &output_path,
                        vtk_version,
                    )?;
                }
            } else {
                write_surface_with_contact_metadata(
                    &surfaces[*i],
                    results,
                    metrics_a,
                    &output_path,
                    vtk_version,
                )?;
            }

            println!("  Output:          {}", output_filename);
            println!();
        }

        // Export metadata if requested
        if let Some(meta) = metadata {
            let metadata_path = output.join("contact_metadata.json");
            meta.export(&metadata_path)?;
            println!("Metadata exported to: {}", metadata_path.display());
            println!();
        }

        // Export sidesets if requested
        if export_sidesets {
            #[cfg(feature = "exodus")]
            {
                println!("Exporting contact sidesets to Exodus file...");

                // Create a copy of the mesh to add sidesets
                let mut mesh_with_sidesets = mesh.clone();

                // Collect all contact surfaces with their sideset names
                let mut contact_surfaces = Vec::new();
                for (part_a, part_b, _results, _metrics_a, _metrics_b, i, j) in
                    detected_pairs.iter()
                {
                    let sideset_name_a = format!("auto_contact_{}", sanitize_filename(part_a));
                    let sideset_name_b = format!("auto_contact_{}", sanitize_filename(part_b));

                    contact_surfaces.push((sideset_name_a, &surfaces[*i]));
                    contact_surfaces.push((sideset_name_b, &surfaces[*j]));
                }

                // Add sidesets to mesh
                add_contact_sidesets_to_mesh(&mut mesh_with_sidesets, &contact_surfaces, &mesh)?;

                // Write mesh with sidesets
                let exodus_output = output.join("mesh_with_contact_sidesets.exo");
                write_exodus(&mesh_with_sidesets, &exodus_output)?;

                println!("Mesh with contact sidesets written to: {}", exodus_output.display());
                println!();
            }
            #[cfg(not(feature = "exodus"))]
            {
                println!("WARNING: --export-sidesets requires exodus feature");
                println!("Skipping sideset export.");
                println!();
            }
        }

        println!("{}", "=".repeat(60));
        println!("Results written to: {}", output.display());
        println!("{}", "=".repeat(60));
    }

    Ok(())
}
