//! Exodus II file reader and writer
//!
//! Exodus II is a NetCDF-based file format for finite element data.
//! This module provides functionality to read and write Exodus II files.

use crate::error::{ContactDetectorError, Result};
use crate::mesh::{HexElement, Mesh, Point};
use std::path::Path;

/// Exodus II file reader
pub struct ExodusReader {
    file: netcdf::File,
}

impl ExodusReader {
    /// Open an Exodus II file for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = netcdf::open(path.as_ref()).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to open file: {}", e))
        })?;

        Ok(Self { file })
    }

    /// Read the complete mesh from the Exodus file
    pub fn read_mesh(&self) -> Result<Mesh> {
        log::info!("Reading Exodus II mesh...");

        let mut mesh = Mesh::new();

        // Read dimensions
        let num_nodes = self.get_dimension("num_nodes")?;
        let num_elem = self.get_dimension("num_elem")?;
        let num_dim = self.get_dimension("num_dim")?;

        log::debug!(
            "Mesh dimensions: {} nodes, {} elements, {} spatial dimensions",
            num_nodes,
            num_elem,
            num_dim
        );

        // Read nodes
        mesh.nodes = self.read_nodes(num_nodes, num_dim)?;
        log::debug!("Read {} nodes", mesh.nodes.len());

        // Read element blocks
        self.read_element_blocks(&mut mesh)?;
        log::debug!(
            "Read {} elements in {} blocks",
            mesh.num_elements(),
            mesh.num_blocks()
        );

        // Read node sets
        self.read_node_sets(&mut mesh)?;
        log::debug!("Read {} node sets", mesh.node_sets.len());

        // Read side sets
        self.read_side_sets(&mut mesh)?;
        log::debug!("Read {} side sets", mesh.side_sets.len());

        log::info!("Successfully read Exodus II mesh");
        Ok(mesh)
    }

    /// Get a dimension value from the file
    fn get_dimension(&self, name: &str) -> Result<usize> {
        self.file.dimension(name).map(|d| d.len()).ok_or_else(|| {
            ContactDetectorError::ExodusReadError(format!("Dimension '{}' not found", name))
        })
    }

    /// Read node coordinates
    fn read_nodes(&self, num_nodes: usize, num_dim: usize) -> Result<Vec<Point>> {
        if num_dim != 3 {
            return Err(ContactDetectorError::ExodusReadError(format!(
                "Only 3D meshes are supported, found {} dimensions",
                num_dim
            )));
        }

        // Read coordinate arrays
        let coordx = self.read_variable_f64("coordx", num_nodes)?;
        let coordy = self.read_variable_f64("coordy", num_nodes)?;
        let coordz = self.read_variable_f64("coordz", num_nodes)?;

        // Combine into points
        let nodes = coordx
            .iter()
            .zip(coordy.iter())
            .zip(coordz.iter())
            .map(|((&x, &y), &z)| Point::new(x, y, z))
            .collect();

        Ok(nodes)
    }

    /// Read all element blocks
    fn read_element_blocks(&self, mesh: &mut Mesh) -> Result<()> {
        let num_el_blk = match self.file.dimension("num_el_blk") {
            Some(dim) => dim.len(),
            None => return Ok(()), // No element blocks
        };

        for blk_id in 1..=num_el_blk {
            self.read_element_block(mesh, blk_id)?;
        }

        Ok(())
    }

    /// Read a single element block
    fn read_element_block(&self, mesh: &mut Mesh, blk_id: usize) -> Result<()> {
        // Get element block metadata
        let connect_var = format!("connect{}", blk_id);
        let var = self.file.variable(&connect_var).ok_or_else(|| {
            ContactDetectorError::ExodusReadError(format!(
                "Element connectivity variable '{}' not found",
                connect_var
            ))
        })?;

        // Get element type from attribute
        let elem_type = var
            .attribute("elem_type")
            .and_then(|attr| attr.value().ok())
            .and_then(|val| {
                if let netcdf::AttributeValue::Str(s) = val {
                    Some(s)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ContactDetectorError::ExodusReadError(format!(
                    "Element type attribute not found for block {}",
                    blk_id
                ))
            })?;

        log::debug!("Reading element block {}: type = {}", blk_id, elem_type);

        // Check if this is a hex block
        let elem_type_upper = elem_type.to_uppercase();
        if !elem_type_upper.starts_with("HEX") && !elem_type_upper.starts_with("HEXAHEDRON") {
            log::warn!(
                "Skipping non-hexahedral block {} (type: {})",
                blk_id,
                elem_type
            );
            return Ok(());
        }

        // Read connectivity array
        let dims = var.dimensions();
        if dims.len() != 2 {
            return Err(ContactDetectorError::ExodusReadError(format!(
                "Expected 2D connectivity array for block {}",
                blk_id
            )));
        }

        let num_elem_in_blk = dims[0].len();
        let num_nodes_per_elem = dims[1].len();

        if num_nodes_per_elem != 8 {
            return Err(ContactDetectorError::InvalidElementType {
                expected: "HEX8 (8 nodes)".to_string(),
                found: format!("{} nodes", num_nodes_per_elem),
            });
        }

        // Read connectivity (Exodus uses 1-based indexing)
        let connectivity_array = var.get(..).map_err(|e| {
            ContactDetectorError::NetcdfError(format!(
                "Failed to read connectivity for block {}: {}",
                blk_id, e
            ))
        })?;
        let connectivity: Vec<i32> = connectivity_array.into_iter().collect();

        // Get block name
        let block_name = self
            .get_block_name(blk_id)
            .unwrap_or_else(|| format!("Block_{}", blk_id));

        // Convert to hex elements
        let block_start_idx = mesh.elements.len();
        for elem_idx in 0..num_elem_in_blk {
            let offset = elem_idx * num_nodes_per_elem;
            let mut node_ids = [0usize; 8];

            for i in 0..8 {
                // Convert from 1-based to 0-based indexing
                let conn_idx = offset + i;
                let node_value = *connectivity.get(conn_idx).ok_or_else(|| {
                    ContactDetectorError::InvalidMeshTopology(format!(
                        "Connectivity index {} out of bounds (block has {} values)",
                        conn_idx,
                        connectivity.len()
                    ))
                })?;

                let node_id = (node_value as usize).checked_sub(1).ok_or_else(|| {
                    ContactDetectorError::InvalidMeshTopology(format!(
                        "Invalid node ID: {} (expected 1-based indexing, got 0)",
                        node_value
                    ))
                })?;
                node_ids[i] = node_id;
            }

            mesh.elements.push(HexElement::new(node_ids));
        }

        // Store block indices
        let block_indices: Vec<usize> = (block_start_idx..mesh.elements.len()).collect();
        mesh.element_blocks.insert(block_name, block_indices);

        Ok(())
    }

    /// Get element block name
    fn get_block_name(&self, blk_id: usize) -> Option<String> {
        // Try to read eb_names variable (stored as character array)
        if let Some(var) = self.file.variable("eb_names") {
            if let Ok(names) = self.read_string_array(&var) {
                return names.get(blk_id - 1).map(|s| s.trim().to_string());
            }
        }

        // Try eb_prop1 (element block IDs)
        if let Some(var) = self.file.variable("eb_prop1") {
            if let Ok(ids_array) = var.get::<i32, _>(..) {
                if let Some(id) = ids_array.into_iter().nth(blk_id - 1) {
                    return Some(format!("Block_{}", id));
                }
            }
        }

        None
    }

    /// Read node sets
    fn read_node_sets(&self, mesh: &mut Mesh) -> Result<()> {
        let num_node_sets = match self.file.dimension("num_node_sets") {
            Some(dim) => dim.len(),
            None => return Ok(()), // No node sets
        };

        for ns_id in 1..=num_node_sets {
            if let Ok(name) = self.get_nodeset_name(ns_id) {
                let var_name = format!("node_ns{}", ns_id);
                if let Some(var) = self.file.variable(&var_name) {
                    if let Ok(nodes_array) = var.get::<i32, _>(..) {
                        // Convert from 1-based to 0-based indexing with validation
                        let node_indices: Result<Vec<usize>> = nodes_array
                            .into_iter()
                            .map(|n| {
                                (n as usize).checked_sub(1).ok_or_else(|| {
                                    ContactDetectorError::InvalidMeshTopology(format!(
                                        "Invalid node ID in node set '{}': {} (expected 1-based indexing)",
                                        name, n
                                    ))
                                })
                            })
                            .collect();

                        match node_indices {
                            Ok(indices) => {
                                mesh.node_sets.insert(name, indices);
                            }
                            Err(e) => {
                                log::warn!("Skipping node set '{}': {}", name, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get node set name
    fn get_nodeset_name(&self, ns_id: usize) -> Result<String> {
        if let Some(var) = self.file.variable("ns_names") {
            if let Ok(names) = self.read_string_array(&var) {
                if let Some(name) = names.get(ns_id - 1) {
                    return Ok(name.trim().to_string());
                }
            }
        }
        Ok(format!("NodeSet_{}", ns_id))
    }

    /// Read side sets
    fn read_side_sets(&self, mesh: &mut Mesh) -> Result<()> {
        let num_side_sets = match self.file.dimension("num_side_sets") {
            Some(dim) => dim.len(),
            None => return Ok(()), // No side sets
        };

        for ss_id in 1..=num_side_sets {
            if let Ok(name) = self.get_sideset_name(ss_id) {
                let elem_var = format!("elem_ss{}", ss_id);
                let side_var = format!("side_ss{}", ss_id);

                if let (Some(elem_v), Some(side_v)) =
                    (self.file.variable(&elem_var), self.file.variable(&side_var))
                {
                    if let (Ok(elems_array), Ok(sides_array)) =
                        (elem_v.get::<i32, _>(..), side_v.get::<i32, _>(..))
                    {
                        // Convert from 1-based to 0-based indexing with validation
                        let side_list: Result<Vec<(usize, u8)>> = elems_array
                            .into_iter()
                            .zip(sides_array.into_iter())
                            .map(|(e, s)| {
                                let elem_id = (e as usize).checked_sub(1).ok_or_else(|| {
                                    ContactDetectorError::InvalidMeshTopology(format!(
                                        "Invalid element ID in side set '{}': {} (expected 1-based indexing)",
                                        name, e
                                    ))
                                })?;
                                Ok((elem_id, s as u8))
                            })
                            .collect();

                        match side_list {
                            Ok(list) => {
                                mesh.side_sets.insert(name, list);
                            }
                            Err(e) => {
                                log::warn!("Skipping side set '{}': {}", name, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get side set name
    fn get_sideset_name(&self, ss_id: usize) -> Result<String> {
        if let Some(var) = self.file.variable("ss_names") {
            if let Ok(names) = self.read_string_array(&var) {
                if let Some(name) = names.get(ss_id - 1) {
                    return Ok(name.trim().to_string());
                }
            }
        }
        Ok(format!("SideSet_{}", ss_id))
    }

    /// Read a float variable as Vec<f64>
    fn read_variable_f64(&self, name: &str, expected_len: usize) -> Result<Vec<f64>> {
        let var = self.file.variable(name).ok_or_else(|| {
            ContactDetectorError::ExodusReadError(format!("Variable '{}' not found", name))
        })?;

        let data_array = var.get(..).map_err(|e| {
            ContactDetectorError::NetcdfError(format!("Failed to read variable '{}': {}", name, e))
        })?;
        let data: Vec<f64> = data_array.into_iter().collect();

        if data.len() != expected_len {
            return Err(ContactDetectorError::ExodusReadError(format!(
                "Variable '{}' has wrong length: expected {}, got {}",
                name,
                expected_len,
                data.len()
            )));
        }

        Ok(data)
    }

    /// Read a string array from a NetCDF variable (stored as 2D char array)
    fn read_string_array(&self, var: &netcdf::Variable) -> Result<Vec<String>> {
        // NetCDF strings are typically stored as 2D char arrays
        // Dimensions: [num_strings, string_length]
        let dims = var.dimensions();

        if dims.is_empty() {
            return Ok(Vec::new());
        }

        if dims.len() == 1 {
            // 1D character array - single string
            let chars_array = var.get(..).map_err(|e| {
                ContactDetectorError::NetcdfError(format!("Failed to read string array: {}", e))
            })?;
            let chars: Vec<u8> = chars_array.into_iter().collect();
            let s = String::from_utf8_lossy(&chars)
                .trim_end_matches('\0')
                .to_string();
            return Ok(vec![s]);
        }

        if dims.len() == 2 {
            // 2D character array - array of strings
            let num_strings = dims[0].len();
            let string_len = dims[1].len();

            let chars_array = var.get(..).map_err(|e| {
                ContactDetectorError::NetcdfError(format!("Failed to read string array: {}", e))
            })?;
            let chars: Vec<u8> = chars_array.into_iter().collect();

            let mut strings = Vec::new();
            for i in 0..num_strings {
                let start = i * string_len;
                let end = start + string_len;
                let string_bytes = &chars[start..end];
                let s = String::from_utf8_lossy(string_bytes)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string();
                strings.push(s);
            }

            return Ok(strings);
        }

        Err(ContactDetectorError::ExodusReadError(format!(
            "Unexpected string array dimensions: {}",
            dims.len()
        )))
    }
}

/// Write a mesh to an Exodus II file
///
/// This is a simplified Exodus writer that writes hex meshes.
/// It creates a basic Exodus file with nodes, elements, and element blocks.
pub fn write_exodus(mesh: &Mesh, output_path: &Path) -> Result<()> {
    log::info!(
        "Writing mesh with {} elements to {:?}",
        mesh.num_elements(),
        output_path
    );

    // Create the file with overwrite mode
    let mut file = netcdf::create(output_path).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to create Exodus file: {}", e))
    })?;

    // Add title
    file.add_attribute("title", "Mesh exported from contact-detector")
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add title attribute: {}", e))
        })?;

    file.add_attribute("api_version", 8.11f32)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add api_version attribute: {}",
                e
            ))
        })?;

    file.add_attribute("version", 8.11f32)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add version attribute: {}", e))
        })?;

    file.add_attribute("floating_point_word_size", 8i32)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add floating_point_word_size attribute: {}",
                e
            ))
        })?;

    file.add_attribute("file_size", 1i32)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add file_size attribute: {}", e))
        })?;

    // Add dimensions
    file.add_dimension("num_dim", 3)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add num_dim dimension: {}", e))
        })?;

    file.add_dimension("num_nodes", mesh.num_nodes())
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add num_nodes dimension: {}", e))
        })?;

    file.add_dimension("num_elem", mesh.num_elements())
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add num_elem dimension: {}", e))
        })?;

    file.add_dimension("num_el_blk", mesh.num_blocks())
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add num_el_blk dimension: {}",
                e
            ))
        })?;

    file.add_dimension("len_string", 33)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add len_string dimension: {}",
                e
            ))
        })?;

    file.add_dimension("num_qa_rec", 0)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add num_qa_rec dimension: {}", e))
        })?;

    file.add_dimension("num_info", 0)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add num_info dimension: {}", e))
        })?;

    file.add_dimension("time_step", 0)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add time_step dimension: {}",
                e
            ))
        })?;

    // Write coordinate arrays
    let coordx: Vec<f64> = mesh.nodes.iter().map(|p| p.x).collect();
    let coordy: Vec<f64> = mesh.nodes.iter().map(|p| p.y).collect();
    let coordz: Vec<f64> = mesh.nodes.iter().map(|p| p.z).collect();

    let mut var = file
        .add_variable::<f64>("coordx", &["num_nodes"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add coordx variable: {}", e))
        })?;
    var.put_values(&coordx, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write coordx data: {}", e))
    })?;

    let mut var = file
        .add_variable::<f64>("coordy", &["num_nodes"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add coordy variable: {}", e))
        })?;
    var.put_values(&coordy, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write coordy data: {}", e))
    })?;

    let mut var = file
        .add_variable::<f64>("coordz", &["num_nodes"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add coordz variable: {}", e))
        })?;
    var.put_values(&coordz, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write coordz data: {}", e))
    })?;

    // Write element blocks
    let mut sorted_blocks: Vec<_> = mesh.element_blocks.iter().collect();
    sorted_blocks.sort_by_key(|(name, _)| *name);

    for (blk_idx, (_block_name, elem_indices)) in sorted_blocks.iter().enumerate() {
        let blk_id = blk_idx + 1;
        let num_elem_in_blk = elem_indices.len();

        // Add dimension for this block
        let dim_name = format!("num_el_in_blk{}", blk_id);
        file.add_dimension(&dim_name, num_elem_in_blk)
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add {} dimension: {}",
                    dim_name, e
                ))
            })?;

        let num_nod_per_el_name = format!("num_nod_per_el{}", blk_id);
        file.add_dimension(&num_nod_per_el_name, 8)
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add {} dimension: {}",
                    num_nod_per_el_name, e
                ))
            })?;

        // Create connectivity variable
        let connect_name = format!("connect{}", blk_id);
        let mut var = file
            .add_variable::<i32>(&connect_name, &[&dim_name, &num_nod_per_el_name])
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add {} variable: {}",
                    connect_name, e
                ))
            })?;

        // Add element type attribute
        var.put_attribute("elem_type", "HEX8")
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add elem_type attribute to {}: {}",
                    connect_name, e
                ))
            })?;

        // Write connectivity (convert to 1-based indexing)
        let mut connectivity = Vec::new();
        for &elem_idx in elem_indices.iter() {
            let elem = &mesh.elements[elem_idx];
            for &node_id in &elem.node_ids {
                connectivity.push((node_id + 1) as i32); // 1-based indexing
            }
        }

        var.put_values(&connectivity, ..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to write connectivity for {}: {}",
                connect_name, e
            ))
        })?;
    }

    // Write element block names
    let max_name_len = 33;
    let num_blocks = mesh.num_blocks();
    let mut eb_names = vec![0u8; num_blocks * max_name_len];

    for (blk_idx, (block_name, _)) in sorted_blocks.iter().enumerate() {
        let start = blk_idx * max_name_len;
        let bytes = block_name.as_bytes();
        let copy_len = bytes.len().min(max_name_len - 1); // Leave room for null terminator
        eb_names[start..start + copy_len].copy_from_slice(&bytes[..copy_len]);
    }

    let mut var = file
        .add_variable::<u8>("eb_names", &["num_el_blk", "len_string"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add eb_names variable: {}", e))
        })?;
    var.put_values(&eb_names, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write eb_names data: {}", e))
    })?;

    // Write side sets if any
    if !mesh.side_sets.is_empty() {
        write_side_sets(&mut file, mesh)?;
    }

    // Write node sets if any
    if !mesh.node_sets.is_empty() {
        write_node_sets(&mut file, mesh)?;
    }

    log::info!("Successfully wrote Exodus file to {:?}", output_path);

    Ok(())
}

/// Write side sets to an Exodus file
fn write_side_sets(file: &mut netcdf::FileMut, mesh: &Mesh) -> Result<()> {
    let num_side_sets = mesh.side_sets.len();

    if num_side_sets == 0 {
        return Ok(());
    }

    log::debug!("Writing {} side sets", num_side_sets);

    // Add num_side_sets dimension
    file.add_dimension("num_side_sets", num_side_sets)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add num_side_sets dimension: {}",
                e
            ))
        })?;

    // Sort side sets for consistent ordering
    let mut sorted_sidesets: Vec<_> = mesh.side_sets.iter().collect();
    sorted_sidesets.sort_by_key(|(name, _)| *name);

    // Write each side set
    for (ss_idx, (ss_name, side_list)) in sorted_sidesets.iter().enumerate() {
        let ss_id = ss_idx + 1;
        let num_sides_in_set = side_list.len();

        log::debug!(
            "Writing side set {}: '{}' with {} sides",
            ss_id,
            ss_name,
            num_sides_in_set
        );

        // Add dimension for this side set
        let dim_name = format!("num_side_ss{}", ss_id);
        file.add_dimension(&dim_name, num_sides_in_set)
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add {} dimension: {}",
                    dim_name, e
                ))
            })?;

        // Convert side list to 1-based indexing
        let elem_ids: Vec<i32> = side_list.iter().map(|(e, _)| (*e + 1) as i32).collect();
        let side_ids: Vec<i32> = side_list.iter().map(|(_, s)| *s as i32).collect();

        // Create element list variable
        let elem_var_name = format!("elem_ss{}", ss_id);
        let mut elem_var = file.add_variable::<i32>(&elem_var_name, &[&dim_name]).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add {} variable: {}",
                elem_var_name, e
            ))
        })?;

        // Write element IDs
        elem_var.put_values(&elem_ids, ..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to write {} data: {}",
                elem_var_name, e
            ))
        })?;

        // Create side list variable
        let side_var_name = format!("side_ss{}", ss_id);
        let mut side_var = file.add_variable::<i32>(&side_var_name, &[&dim_name]).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add {} variable: {}",
                side_var_name, e
            ))
        })?;

        // Write side IDs
        side_var.put_values(&side_ids, ..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to write {} data: {}",
                side_var_name, e
            ))
        })?;
    }

    // Write side set names
    let max_name_len = 33;
    let mut ss_names = vec![0u8; num_side_sets * max_name_len];

    for (ss_idx, (ss_name, _)) in sorted_sidesets.iter().enumerate() {
        let start = ss_idx * max_name_len;
        let bytes = ss_name.as_bytes();
        let copy_len = bytes.len().min(max_name_len - 1);
        ss_names[start..start + copy_len].copy_from_slice(&bytes[..copy_len]);
    }

    let mut var = file
        .add_variable::<u8>("ss_names", &["num_side_sets", "len_string"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add ss_names variable: {}", e))
        })?;
    var.put_values(&ss_names, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write ss_names data: {}", e))
    })?;

    Ok(())
}

/// Write node sets to an Exodus file
fn write_node_sets(file: &mut netcdf::FileMut, mesh: &Mesh) -> Result<()> {
    let num_node_sets = mesh.node_sets.len();

    if num_node_sets == 0 {
        return Ok(());
    }

    log::debug!("Writing {} node sets", num_node_sets);

    // Add num_node_sets dimension
    file.add_dimension("num_node_sets", num_node_sets)
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to add num_node_sets dimension: {}",
                e
            ))
        })?;

    // Sort node sets for consistent ordering
    let mut sorted_nodesets: Vec<_> = mesh.node_sets.iter().collect();
    sorted_nodesets.sort_by_key(|(name, _)| *name);

    // Write each node set
    for (ns_idx, (ns_name, node_list)) in sorted_nodesets.iter().enumerate() {
        let ns_id = ns_idx + 1;
        let num_nodes_in_set = node_list.len();

        log::debug!(
            "Writing node set {}: '{}' with {} nodes",
            ns_id,
            ns_name,
            num_nodes_in_set
        );

        // Add dimension for this node set
        let dim_name = format!("num_nod_ns{}", ns_id);
        file.add_dimension(&dim_name, num_nodes_in_set)
            .map_err(|e| {
                ContactDetectorError::ExodusReadError(format!(
                    "Failed to add {} dimension: {}",
                    dim_name, e
                ))
            })?;

        // Create node list variable
        let var_name = format!("node_ns{}", ns_id);
        let mut var = file.add_variable::<i32>(&var_name, &[&dim_name]).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add {} variable: {}", var_name, e))
        })?;

        // Convert node list to 1-based indexing
        let node_ids: Vec<i32> = node_list.iter().map(|n| (*n + 1) as i32).collect();

        // Write node IDs
        var.put_values(&node_ids, ..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to write {} data: {}", var_name, e))
        })?;
    }

    // Write node set names
    let max_name_len = 33;
    let mut ns_names = vec![0u8; num_node_sets * max_name_len];

    for (ns_idx, (ns_name, _)) in sorted_nodesets.iter().enumerate() {
        let start = ns_idx * max_name_len;
        let bytes = ns_name.as_bytes();
        let copy_len = bytes.len().min(max_name_len - 1);
        ns_names[start..start + copy_len].copy_from_slice(&bytes[..copy_len]);
    }

    let mut var = file
        .add_variable::<u8>("ns_names", &["num_node_sets", "len_string"])
        .map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to add ns_names variable: {}", e))
        })?;
    var.put_values(&ns_names, ..).map_err(|e| {
        ContactDetectorError::ExodusReadError(format!("Failed to write ns_names data: {}", e))
    })?;

    Ok(())
}

/// Convert contact surface faces to sideset format (element_id, face_id pairs)
///
/// This function maps surface faces from contact detection back to the original
/// hexahedral mesh elements and their face IDs for Exodus II sideset export.
pub fn surface_to_sideset(
    surface: &crate::mesh::SurfaceMesh,
    mesh: &Mesh,
) -> Result<Vec<(usize, u8)>> {
    use std::collections::HashMap;

    log::debug!(
        "Converting surface '{}' with {} faces to sideset format",
        surface.part_name,
        surface.faces.len()
    );

    // Build a map from canonical face to (element_idx, face_id)
    let mut face_to_elem_and_id: HashMap<crate::mesh::QuadFace, (usize, u8)> = HashMap::new();

    for (elem_idx, element) in mesh.elements.iter().enumerate() {
        let hex_faces = element.faces();
        for (face_id, face) in hex_faces.iter().enumerate() {
            let canonical = face.canonical();
            face_to_elem_and_id.insert(canonical, (elem_idx, face_id as u8));
        }
    }

    // Map each surface face to (element_idx, face_id)
    let mut sideset = Vec::new();

    for face in &surface.faces {
        let canonical = face.canonical();

        if let Some(&(elem_idx, face_id)) = face_to_elem_and_id.get(&canonical) {
            sideset.push((elem_idx, face_id));
        } else {
            log::warn!(
                "Surface face with nodes {:?} not found in mesh",
                face.node_ids
            );
        }
    }

    log::debug!("Mapped {} surface faces to sideset", sideset.len());

    Ok(sideset)
}

/// Add contact surface sidesets to a mesh
///
/// This function takes a mesh and adds sidesets for detected contact surfaces.
/// The sidesets are named using the format "auto_contact_{surface_name}".
pub fn add_contact_sidesets_to_mesh(
    mesh: &mut Mesh,
    contact_surfaces: &[(String, &crate::mesh::SurfaceMesh)],
    original_mesh: &Mesh,
) -> Result<()> {
    for (sideset_name, surface) in contact_surfaces {
        log::info!("Adding sideset '{}' for surface '{}'", sideset_name, surface.part_name);

        let sideset = surface_to_sideset(surface, original_mesh)?;

        if !sideset.is_empty() {
            mesh.side_sets.insert(sideset_name.clone(), sideset);
        } else {
            log::warn!("Skipping empty sideset '{}'", sideset_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{HexElement, Point, QuadFace, SurfaceMesh, Vec3};

    #[test]
    #[ignore] // Only run when test file is available
    fn test_read_exodus_file() {
        let reader = ExodusReader::open("test-data/hexcyl.exo").unwrap();
        let mesh = reader.read_mesh().unwrap();

        assert!(mesh.num_nodes() > 0);
        assert!(mesh.num_elements() > 0);
        println!("Nodes: {}", mesh.num_nodes());
        println!("Elements: {}", mesh.num_elements());
        println!("Blocks: {}", mesh.num_blocks());
    }

    #[test]
    fn test_surface_to_sideset() {
        // Create a simple mesh with one hex element
        let mut mesh = Mesh::new();
        mesh.nodes = vec![
            Point::new(0.0, 0.0, 0.0), // 0
            Point::new(1.0, 0.0, 0.0), // 1
            Point::new(1.0, 1.0, 0.0), // 2
            Point::new(0.0, 1.0, 0.0), // 3
            Point::new(0.0, 0.0, 1.0), // 4
            Point::new(1.0, 0.0, 1.0), // 5
            Point::new(1.0, 1.0, 1.0), // 6
            Point::new(0.0, 1.0, 1.0), // 7
        ];
        mesh.elements = vec![HexElement::new([0, 1, 2, 3, 4, 5, 6, 7])];
        mesh.element_blocks
            .insert("Block1".to_string(), vec![0]);

        // Create a surface with the top face
        let mut surface = SurfaceMesh::new("Block1".to_string());
        surface.faces = vec![QuadFace::new([4, 5, 6, 7])];
        surface.face_normals = vec![Vec3::new(0.0, 0.0, 1.0)];
        surface.face_areas = vec![1.0];
        surface.nodes = mesh.nodes.clone();

        // Convert to sideset
        let result = surface_to_sideset(&surface, &mesh);
        assert!(result.is_ok());

        let sideset = result.unwrap();
        assert_eq!(sideset.len(), 1);
        assert_eq!(sideset[0].0, 0); // element 0
        assert_eq!(sideset[0].1, 1); // face 1 (top face)
    }

    #[test]
    fn test_add_contact_sidesets_to_mesh() {
        // Create a simple mesh
        let mut mesh = Mesh::new();
        mesh.nodes = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
            Point::new(0.0, 0.0, 1.0),
            Point::new(1.0, 0.0, 1.0),
            Point::new(1.0, 1.0, 1.0),
            Point::new(0.0, 1.0, 1.0),
        ];
        mesh.elements = vec![HexElement::new([0, 1, 2, 3, 4, 5, 6, 7])];
        mesh.element_blocks
            .insert("Block1".to_string(), vec![0]);

        // Create a surface
        let mut surface = SurfaceMesh::new("Block1:patch_1".to_string());
        surface.faces = vec![QuadFace::new([4, 5, 6, 7])];
        surface.face_normals = vec![Vec3::new(0.0, 0.0, 1.0)];
        surface.face_areas = vec![1.0];
        surface.nodes = mesh.nodes.clone();

        // Clone the original mesh
        let original_mesh = mesh.clone();

        // Add sidesets
        let contact_surfaces = vec![("auto_contact_Block1_patch_1".to_string(), &surface)];
        let result = add_contact_sidesets_to_mesh(&mut mesh, &contact_surfaces, &original_mesh);
        assert!(result.is_ok());

        // Verify sideset was added
        assert_eq!(mesh.side_sets.len(), 1);
        assert!(mesh.side_sets.contains_key("auto_contact_Block1_patch_1"));

        let sideset = &mesh.side_sets["auto_contact_Block1_patch_1"];
        assert_eq!(sideset.len(), 1);
        assert_eq!(sideset[0].0, 0); // element 0
    }

    #[test]
    fn test_write_exodus_with_sidesets() {
        // Create a mesh with sidesets
        let mut mesh = Mesh::new();
        mesh.nodes = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
            Point::new(0.0, 0.0, 1.0),
            Point::new(1.0, 0.0, 1.0),
            Point::new(1.0, 1.0, 1.0),
            Point::new(0.0, 1.0, 1.0),
        ];
        mesh.elements = vec![HexElement::new([0, 1, 2, 3, 4, 5, 6, 7])];
        mesh.element_blocks
            .insert("Block1".to_string(), vec![0]);

        // Add a sideset
        mesh.side_sets
            .insert("test_sideset".to_string(), vec![(0, 1)]);

        // Write to file
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_mesh_with_sidesets.exo");

        let result = write_exodus(&mesh, &output_path);
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn test_write_exodus_with_nodesets() {
        // Create a mesh with nodesets
        let mut mesh = Mesh::new();
        mesh.nodes = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
            Point::new(0.0, 0.0, 1.0),
            Point::new(1.0, 0.0, 1.0),
            Point::new(1.0, 1.0, 1.0),
            Point::new(0.0, 1.0, 1.0),
        ];
        mesh.elements = vec![HexElement::new([0, 1, 2, 3, 4, 5, 6, 7])];
        mesh.element_blocks
            .insert("Block1".to_string(), vec![0]);

        // Add a nodeset
        mesh.node_sets
            .insert("test_nodeset".to_string(), vec![0, 1, 2, 3]);

        // Write to file
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_mesh_with_nodesets.exo");

        let result = write_exodus(&mesh, &output_path);
        assert!(result.is_ok());

        // Verify file was created
        assert!(output_path.exists());

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }

    #[test]
    fn test_surface_to_sideset_multiple_faces() {
        // Create a mesh with multiple elements
        let mut mesh = Mesh::new();

        // Create nodes for 2 hex elements stacked vertically
        mesh.nodes = vec![
            // First hex (bottom)
            Point::new(0.0, 0.0, 0.0), // 0
            Point::new(1.0, 0.0, 0.0), // 1
            Point::new(1.0, 1.0, 0.0), // 2
            Point::new(0.0, 1.0, 0.0), // 3
            Point::new(0.0, 0.0, 1.0), // 4
            Point::new(1.0, 0.0, 1.0), // 5
            Point::new(1.0, 1.0, 1.0), // 6
            Point::new(0.0, 1.0, 1.0), // 7
            // Second hex (top)
            Point::new(0.0, 0.0, 2.0), // 8
            Point::new(1.0, 0.0, 2.0), // 9
            Point::new(1.0, 1.0, 2.0), // 10
            Point::new(0.0, 1.0, 2.0), // 11
        ];

        mesh.elements = vec![
            HexElement::new([0, 1, 2, 3, 4, 5, 6, 7]),
            HexElement::new([4, 5, 6, 7, 8, 9, 10, 11]),
        ];
        mesh.element_blocks
            .insert("Block1".to_string(), vec![0, 1]);

        // Create a surface with bottom and top faces
        let mut surface = SurfaceMesh::new("Block1".to_string());
        surface.faces = vec![
            QuadFace::new([0, 3, 2, 1]), // bottom face of first hex
            QuadFace::new([8, 9, 10, 11]), // top face of second hex
        ];
        surface.face_normals = vec![Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0)];
        surface.face_areas = vec![1.0, 1.0];
        surface.nodes = mesh.nodes.clone();

        // Convert to sideset
        let result = surface_to_sideset(&surface, &mesh);
        assert!(result.is_ok());

        let sideset = result.unwrap();
        assert_eq!(sideset.len(), 2);

        // Verify both faces were mapped
        assert!(sideset.iter().any(|(elem, face)| *elem == 0 && *face == 0));
        assert!(sideset.iter().any(|(elem, face)| *elem == 1 && *face == 1));
    }
}
