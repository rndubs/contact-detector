//! Exodus II file reader
//!
//! Exodus II is a NetCDF-based file format for finite element data.
//! This module provides functionality to read Exodus II files and convert them to our internal mesh representation.

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
        log::debug!("Read {} elements in {} blocks", mesh.num_elements(), mesh.num_blocks());

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
        self.file
            .dimension(name)
            .map(|d| d.len())
            .ok_or_else(|| {
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
                if let netcdf::AttrValue::Str(s) = val {
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
            log::warn!("Skipping non-hexahedral block {} (type: {})", blk_id, elem_type);
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
        let connectivity: Vec<i32> = var.get(..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!(
                "Failed to read connectivity for block {}: {}",
                blk_id, e
            ))
        })?;

        // Get block name
        let block_name = self.get_block_name(blk_id).unwrap_or_else(|| format!("Block_{}", blk_id));

        // Convert to hex elements
        let block_start_idx = mesh.elements.len();
        for elem_idx in 0..num_elem_in_blk {
            let offset = elem_idx * num_nodes_per_elem;
            let mut node_ids = [0usize; 8];

            for i in 0..8 {
                // Convert from 1-based to 0-based indexing
                let node_id = connectivity[offset + i] as usize - 1;
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
        // Try to read eb_names variable
        if let Some(var) = self.file.variable("eb_names") {
            if let Ok(names) = var.get::<String, _>(..) {
                return names.get(blk_id - 1).map(|s| s.trim().to_string());
            }
        }

        // Try eb_prop1 (element block IDs)
        if let Some(var) = self.file.variable("eb_prop1") {
            if let Ok(ids) = var.get::<i32, _>(..) {
                if let Some(&id) = ids.get(blk_id - 1) {
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
                    if let Ok(nodes) = var.get::<i32, _>(..) {
                        // Convert from 1-based to 0-based indexing
                        let node_indices: Vec<usize> = nodes.iter().map(|&n| (n - 1) as usize).collect();
                        mesh.node_sets.insert(name, node_indices);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get node set name
    fn get_nodeset_name(&self, ns_id: usize) -> Result<String> {
        if let Some(var) = self.file.variable("ns_names") {
            if let Ok(names) = var.get::<String, _>(..) {
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

                if let (Some(elem_v), Some(side_v)) = (self.file.variable(&elem_var), self.file.variable(&side_var)) {
                    if let (Ok(elems), Ok(sides)) = (elem_v.get::<i32, _>(..), side_v.get::<i32, _>(..())) {
                        let side_list: Vec<(usize, u8)> = elems
                            .iter()
                            .zip(sides.iter())
                            .map(|(&e, &s)| ((e - 1) as usize, s as u8))
                            .collect();
                        mesh.side_sets.insert(name, side_list);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get side set name
    fn get_sideset_name(&self, ss_id: usize) -> Result<String> {
        if let Some(var) = self.file.variable("ss_names") {
            if let Ok(names) = var.get::<String, _>(..) {
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

        let data: Vec<f64> = var.get(..).map_err(|e| {
            ContactDetectorError::ExodusReadError(format!("Failed to read variable '{}': {}", name, e))
        })?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
