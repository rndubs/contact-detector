//! Core mesh data structures

use nalgebra::{Point3, Vector3};
use std::collections::HashMap;

/// 3D point type
pub type Point = Point3<f64>;

/// 3D vector type
pub type Vec3 = Vector3<f64>;

/// Hexahedral element with 8 nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HexElement {
    /// Node IDs in canonical ordering (0-7)
    /// Ordering follows Exodus II convention:
    ///   Bottom face (z-): 0,1,2,3 (counter-clockwise)
    ///   Top face (z+):    4,5,6,7 (counter-clockwise)
    pub node_ids: [usize; 8],
}

impl HexElement {
    /// Create a new hex element
    pub fn new(node_ids: [usize; 8]) -> Self {
        Self { node_ids }
    }

    /// Get the 6 quad faces of this hex element
    /// Returns faces in order: bottom, top, front, right, back, left
    /// Each face has 4 node IDs in counter-clockwise order when viewed from outside
    pub fn faces(&self) -> [QuadFace; 6] {
        let n = self.node_ids;
        [
            QuadFace::new([n[0], n[3], n[2], n[1]]), // bottom (z-)
            QuadFace::new([n[4], n[5], n[6], n[7]]), // top (z+)
            QuadFace::new([n[0], n[1], n[5], n[4]]), // front (y-)
            QuadFace::new([n[1], n[2], n[6], n[5]]), // right (x+)
            QuadFace::new([n[2], n[3], n[7], n[6]]), // back (y+)
            QuadFace::new([n[3], n[0], n[4], n[7]]), // left (x-)
        ]
    }
}

/// Quadrilateral face with 4 nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuadFace {
    /// Node IDs in counter-clockwise order
    pub node_ids: [usize; 4],
}

impl QuadFace {
    /// Create a new quad face
    pub fn new(node_ids: [usize; 4]) -> Self {
        Self { node_ids }
    }

    /// Get canonical form for hashing (smallest rotation)
    /// This ensures two faces with the same nodes in different orders hash equally
    pub fn canonical(&self) -> Self {
        let mut nodes = self.node_ids;

        // Find the minimum starting index
        let min_idx = nodes
            .iter()
            .enumerate()
            .min_by_key(|(_, &n)| n)
            .map(|(i, _)| i)
            .unwrap();

        // Rotate to start with minimum node
        nodes.rotate_left(min_idx);

        // Check if we need to reverse (for faces that might be flipped)
        // We want the lexicographically smallest representation
        let reversed = [nodes[0], nodes[3], nodes[2], nodes[1]];
        if reversed[1..] < nodes[1..] {
            QuadFace::new(reversed)
        } else {
            QuadFace::new(nodes)
        }
    }
}

/// Complete mesh representation
#[derive(Debug, Clone)]
pub struct Mesh {
    /// All nodes in the mesh
    pub nodes: Vec<Point>,

    /// All hexahedral elements
    pub elements: Vec<HexElement>,

    /// Element blocks grouped by part/material name
    /// Maps block name -> element indices
    pub element_blocks: HashMap<String, Vec<usize>>,

    /// Node sets (named groups of nodes)
    /// Maps nodeset name -> node indices
    pub node_sets: HashMap<String, Vec<usize>>,

    /// Side sets (named groups of element faces)
    /// Maps sideset name -> (element index, local face id)
    pub side_sets: HashMap<String, Vec<(usize, u8)>>,
}

impl Mesh {
    /// Create a new empty mesh
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            elements: Vec::new(),
            element_blocks: HashMap::new(),
            node_sets: HashMap::new(),
            side_sets: HashMap::new(),
        }
    }

    /// Get total number of nodes
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get total number of elements
    pub fn num_elements(&self) -> usize {
        self.elements.len()
    }

    /// Get number of element blocks
    pub fn num_blocks(&self) -> usize {
        self.element_blocks.len()
    }

    /// Get elements in a specific block
    pub fn get_block(&self, name: &str) -> Option<Vec<&HexElement>> {
        self.element_blocks.get(name).map(|indices| {
            indices.iter().map(|&idx| &self.elements[idx]).collect()
        })
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Surface mesh (extracted from volume mesh)
#[derive(Debug, Clone)]
pub struct SurfaceMesh {
    /// Part/block name this surface belongs to
    pub part_name: String,

    /// Surface faces (subset of volume mesh faces)
    pub faces: Vec<QuadFace>,

    /// Face normals (outward pointing)
    pub face_normals: Vec<Vec3>,

    /// Face centroids
    pub face_centroids: Vec<Point>,

    /// Face areas
    pub face_areas: Vec<f64>,

    /// Reference to original nodes (shared with volume mesh)
    pub nodes: Vec<Point>,
}

impl SurfaceMesh {
    /// Create a new surface mesh
    pub fn new(part_name: String) -> Self {
        Self {
            part_name,
            faces: Vec::new(),
            face_normals: Vec::new(),
            face_centroids: Vec::new(),
            face_areas: Vec::new(),
            nodes: Vec::new(),
        }
    }

    /// Get number of faces in this surface
    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }

    /// Get total surface area
    pub fn total_area(&self) -> f64 {
        self.face_areas.iter().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_faces() {
        let hex = HexElement::new([0, 1, 2, 3, 4, 5, 6, 7]);
        let faces = hex.faces();

        assert_eq!(faces.len(), 6);
        assert_eq!(faces[0].node_ids, [0, 3, 2, 1]); // bottom
        assert_eq!(faces[1].node_ids, [4, 5, 6, 7]); // top
    }

    #[test]
    fn test_quad_canonical() {
        let face1 = QuadFace::new([1, 2, 3, 4]);
        let face2 = QuadFace::new([2, 3, 4, 1]); // rotated
        let _face3 = QuadFace::new([4, 3, 2, 1]); // reversed

        assert_eq!(face1.canonical(), face2.canonical());
        // Note: face3 reversed should also match after canonicalization
    }

    #[test]
    fn test_mesh_creation() {
        let mut mesh = Mesh::new();
        assert_eq!(mesh.num_nodes(), 0);
        assert_eq!(mesh.num_elements(), 0);

        mesh.nodes.push(Point::new(0.0, 0.0, 0.0));
        assert_eq!(mesh.num_nodes(), 1);
    }
}
