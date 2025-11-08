//! Synthetic mesh generation utilities for benchmarking
//!
//! This module provides functions to generate hexahedral meshes of various sizes
//! for performance testing, since we only have a 3.7K element test file.

use contact_detector::mesh::types::{HexElement, Mesh, Point};
use std::collections::HashMap;

/// Generate a structured 3D grid of hexahedral elements
///
/// Creates a rectangular grid with the specified number of elements in each direction.
/// Total elements = nx * ny * nz
///
/// # Arguments
/// * `nx` - Number of elements in X direction
/// * `ny` - Number of elements in Y direction
/// * `nz` - Number of elements in Z direction
/// * `element_size` - Size of each hex element (cube edge length)
///
/// # Returns
/// A Mesh containing (nx * ny * nz) hex elements
pub fn generate_hex_grid(nx: usize, ny: usize, nz: usize, element_size: f64) -> Mesh {
    let num_nodes_x = nx + 1;
    let num_nodes_y = ny + 1;
    let num_nodes_z = nz + 1;
    let total_nodes = num_nodes_x * num_nodes_y * num_nodes_z;
    let total_elements = nx * ny * nz;

    // Pre-allocate nodes
    let mut nodes = Vec::with_capacity(total_nodes);

    // Generate node grid
    for k in 0..num_nodes_z {
        for j in 0..num_nodes_y {
            for i in 0..num_nodes_x {
                let x = i as f64 * element_size;
                let y = j as f64 * element_size;
                let z = k as f64 * element_size;
                nodes.push(Point::new(x, y, z));
            }
        }
    }

    // Pre-allocate elements
    let mut elements = Vec::with_capacity(total_elements);

    // Generate hex elements
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                // Calculate node indices for this hex
                // Bottom face (z=k)
                let n0 = node_index(i, j, k, num_nodes_x, num_nodes_y);
                let n1 = node_index(i + 1, j, k, num_nodes_x, num_nodes_y);
                let n2 = node_index(i + 1, j + 1, k, num_nodes_x, num_nodes_y);
                let n3 = node_index(i, j + 1, k, num_nodes_x, num_nodes_y);

                // Top face (z=k+1)
                let n4 = node_index(i, j, k + 1, num_nodes_x, num_nodes_y);
                let n5 = node_index(i + 1, j, k + 1, num_nodes_x, num_nodes_y);
                let n6 = node_index(i + 1, j + 1, k + 1, num_nodes_x, num_nodes_y);
                let n7 = node_index(i, j + 1, k + 1, num_nodes_x, num_nodes_y);

                let hex = HexElement::new([n0, n1, n2, n3, n4, n5, n6, n7]);
                elements.push(hex);
            }
        }
    }

    // Create element blocks
    let mut element_blocks = HashMap::new();
    let all_element_indices: Vec<usize> = (0..total_elements).collect();
    element_blocks.insert("Block1".to_string(), all_element_indices);

    Mesh {
        nodes,
        elements,
        element_blocks,
        node_sets: HashMap::new(),
        side_sets: HashMap::new(),
    }
}

/// Helper to calculate linear node index from 3D grid coordinates
#[inline]
fn node_index(i: usize, j: usize, k: usize, nx: usize, ny: usize) -> usize {
    k * nx * ny + j * nx + i
}

/// Generate a hex grid with small perturbations to avoid k-d tree issues
///
/// This is useful for benchmarking when you need unique positions to prevent
/// k-d tree construction errors due to duplicate coordinates.
fn generate_hex_grid_with_perturbation(
    nx: usize,
    ny: usize,
    nz: usize,
    element_size: f64,
    z_offset: f64,
) -> Mesh {
    let num_nodes_x = nx + 1;
    let num_nodes_y = ny + 1;
    let num_nodes_z = nz + 1;
    let total_nodes = num_nodes_x * num_nodes_y * num_nodes_z;
    let total_elements = nx * ny * nz;

    // Pre-allocate nodes
    let mut nodes = Vec::with_capacity(total_nodes);

    // Generate node grid with small perturbations
    for k in 0..num_nodes_z {
        for j in 0..num_nodes_y {
            for i in 0..num_nodes_x {
                let x = i as f64 * element_size;
                let y = j as f64 * element_size;
                let z = k as f64 * element_size + z_offset;

                // Add tiny perturbation based on node index to ensure uniqueness
                let node_idx = node_index(i, j, k, num_nodes_x, num_nodes_y);
                let perturbation = 0.0001 * element_size;
                let px = (node_idx as f64 * 0.123) % perturbation;
                let py = (node_idx as f64 * 0.456) % perturbation;
                let pz = (node_idx as f64 * 0.789) % perturbation;

                nodes.push(Point::new(x + px, y + py, z + pz));
            }
        }
    }

    // Pre-allocate elements
    let mut elements = Vec::with_capacity(total_elements);

    // Generate hex elements
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                // Calculate node indices for this hex
                let n0 = node_index(i, j, k, num_nodes_x, num_nodes_y);
                let n1 = node_index(i + 1, j, k, num_nodes_x, num_nodes_y);
                let n2 = node_index(i + 1, j + 1, k, num_nodes_x, num_nodes_y);
                let n3 = node_index(i, j + 1, k, num_nodes_x, num_nodes_y);

                let n4 = node_index(i, j, k + 1, num_nodes_x, num_nodes_y);
                let n5 = node_index(i + 1, j, k + 1, num_nodes_x, num_nodes_y);
                let n6 = node_index(i + 1, j + 1, k + 1, num_nodes_x, num_nodes_y);
                let n7 = node_index(i, j + 1, k + 1, num_nodes_x, num_nodes_y);

                let hex = HexElement::new([n0, n1, n2, n3, n4, n5, n6, n7]);
                elements.push(hex);
            }
        }
    }

    // Create element blocks
    let mut element_blocks = HashMap::new();
    let all_element_indices: Vec<usize> = (0..total_elements).collect();
    element_blocks.insert("Block1".to_string(), all_element_indices);

    Mesh {
        nodes,
        elements,
        element_blocks,
        node_sets: HashMap::new(),
        side_sets: HashMap::new(),
    }
}

/// Generate two parallel surfaces separated by a gap
///
/// This creates two meshes that face each other with a specified gap distance,
/// useful for contact detection benchmarking.
///
/// # Arguments
/// * `nx` - Number of elements in X direction
/// * `ny` - Number of elements in Y direction
/// * `gap` - Gap distance between the two surfaces
/// * `element_size` - Size of each hex element
///
/// # Returns
/// A tuple of (surface_a_mesh, surface_b_mesh)
pub fn generate_parallel_surfaces(
    nx: usize,
    ny: usize,
    gap: f64,
    element_size: f64,
) -> (Mesh, Mesh) {
    // Generate first surface with slight perturbations to avoid k-d tree issues
    let mesh_a = generate_hex_grid_with_perturbation(nx, ny, 1, element_size, 0.0);

    // Generate second surface (offset in Z direction by gap + element_size)
    let z_offset = element_size + gap;
    let mut mesh_b = generate_hex_grid_with_perturbation(nx, ny, 1, element_size, z_offset);

    // Update block name for mesh B
    let elements_b: Vec<usize> = (0..mesh_b.num_elements()).collect();
    mesh_b.element_blocks.clear();
    mesh_b.element_blocks.insert("Block2".to_string(), elements_b);

    (mesh_a, mesh_b)
}

/// Calculate mesh sizes for target element counts
///
/// Returns (nx, ny, nz) that approximately achieve the target element count
pub fn calculate_grid_dimensions(target_elements: usize) -> (usize, usize, usize) {
    // Try to create a roughly cubic mesh
    let cube_root = (target_elements as f64).powf(1.0 / 3.0).ceil() as usize;
    let nx = cube_root;
    let ny = cube_root;
    let nz = cube_root;

    // Adjust to get closer to target
    let actual = nx * ny * nz;
    if actual > target_elements {
        // Reduce one dimension slightly
        let nz_adjusted = target_elements / (nx * ny);
        (nx, ny, nz_adjusted.max(1))
    } else {
        (nx, ny, nz)
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_grid_dimensions, generate_hex_grid, generate_parallel_surfaces};

    #[test]
    fn test_generate_small_grid() {
        let mesh = generate_hex_grid(2, 2, 2, 1.0);
        assert_eq!(mesh.num_elements(), 8); // 2*2*2
        assert_eq!(mesh.num_nodes(), 27); // 3*3*3
    }

    #[test]
    fn test_parallel_surfaces() {
        let (mesh_a, mesh_b) = generate_parallel_surfaces(10, 10, 0.001, 1.0);
        assert_eq!(mesh_a.num_elements(), 100); // 10*10*1
        assert_eq!(mesh_b.num_elements(), 100); // 10*10*1
    }

    #[test]
    fn test_calculate_dimensions() {
        let (nx, ny, nz) = calculate_grid_dimensions(1000);
        let actual = nx * ny * nz;
        // Should be close to target (within 10%)
        assert!((actual as f64 - 1000.0).abs() / 1000.0 < 0.1);
    }
}
