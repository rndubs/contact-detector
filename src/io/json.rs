//! Simple JSON mesh format for testing (alternative to Exodus when HDF5 unavailable)

use crate::error::{ContactDetectorError, Result};
use crate::mesh::{HexElement, Mesh, Point};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct JsonMesh {
    nodes: Vec<[f64; 3]>,
    elements: Vec<[usize; 8]>,
    #[serde(default)]
    element_blocks: HashMap<String, Vec<usize>>,
    #[serde(default)]
    node_sets: HashMap<String, Vec<usize>>,
    #[serde(default)]
    side_sets: HashMap<String, Vec<(usize, u8)>>,
}

pub fn read_json_mesh<P: AsRef<Path>>(path: P) -> Result<Mesh> {
    let file = File::open(path.as_ref()).map_err(|e| {
        ContactDetectorError::IoError(e)
    })?;

    let reader = BufReader::new(file);
    let json_mesh: JsonMesh = serde_json::from_reader(reader).map_err(|e| {
        ContactDetectorError::ConfigError(format!("Failed to parse JSON mesh: {}", e))
    })?;

    let mut mesh = Mesh::new();

    // Convert nodes
    mesh.nodes = json_mesh
        .nodes
        .into_iter()
        .map(|[x, y, z]| Point::new(x, y, z))
        .collect();

    // Convert elements
    mesh.elements = json_mesh
        .elements
        .into_iter()
        .map(|nodes| HexElement::new(nodes))
        .collect();

    // Copy metadata
    mesh.element_blocks = json_mesh.element_blocks;
    mesh.node_sets = json_mesh.node_sets;
    mesh.side_sets = json_mesh.side_sets;

    Ok(mesh)
}

pub fn write_json_mesh<P: AsRef<Path>>(mesh: &Mesh, path: P) -> Result<()> {
    let json_mesh = JsonMesh {
        nodes: mesh.nodes.iter().map(|p| [p.x, p.y, p.z]).collect(),
        elements: mesh.elements.iter().map(|e| e.node_ids).collect(),
        element_blocks: mesh.element_blocks.clone(),
        node_sets: mesh.node_sets.clone(),
        side_sets: mesh.side_sets.clone(),
    };

    let file = File::create(path.as_ref())?;
    serde_json::to_writer_pretty(file, &json_mesh).map_err(|e| {
        ContactDetectorError::ConfigError(format!("Failed to write JSON mesh: {}", e))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
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

        let path = "/tmp/test_mesh.json";
        write_json_mesh(&mesh, path).unwrap();
        let loaded = read_json_mesh(path).unwrap();

        assert_eq!(loaded.num_nodes(), 8);
        assert_eq!(loaded.num_elements(), 1);
        assert_eq!(loaded.num_blocks(), 1);
    }
}
