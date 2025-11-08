//! Surface extraction ("skinning") from hexahedral mesh

use crate::error::{ContactDetectorError, Result};
use crate::mesh::geometry::{compute_face_area, compute_face_centroid, compute_face_normal};
use crate::mesh::types::{Mesh, Point, QuadFace, SurfaceMesh};
use std::collections::HashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Extract surface mesh from a volume mesh
/// Returns one SurfaceMesh per element block (part)
pub fn extract_surface(mesh: &Mesh) -> Result<Vec<SurfaceMesh>> {
    log::info!(
        "Extracting surface from mesh with {} elements",
        mesh.num_elements()
    );

    // Build face adjacency map
    let face_adjacency = build_face_adjacency(mesh)?;

    // Extract boundary faces (faces with exactly 1 adjacent element)
    let boundary_faces = extract_boundary_faces(&face_adjacency);

    log::info!("Found {} boundary faces", boundary_faces.len());

    // Group faces by element block
    let surfaces = group_by_block(mesh, &boundary_faces, &face_adjacency)?;

    log::info!("Created {} surface meshes", surfaces.len());

    Ok(surfaces)
}

/// Build a map from canonical faces to the elements that contain them
fn build_face_adjacency(mesh: &Mesh) -> Result<HashMap<QuadFace, Vec<usize>>> {
    let mut adjacency: HashMap<QuadFace, Vec<usize>> = HashMap::new();

    for (elem_idx, element) in mesh.elements.iter().enumerate() {
        let faces = element.faces();

        for face in &faces {
            // Use canonical form for consistent hashing
            let canonical_face = face.canonical();
            adjacency
                .entry(canonical_face)
                .or_default()
                .push(elem_idx);
        }
    }

    Ok(adjacency)
}

/// Extract boundary faces (faces with exactly one adjacent element)
fn extract_boundary_faces(
    face_adjacency: &HashMap<QuadFace, Vec<usize>>,
) -> HashMap<QuadFace, usize> {
    let mut boundary_faces = HashMap::new();

    for (face, elements) in face_adjacency {
        if elements.len() == 1 {
            // This is a boundary face - only one element adjacent
            boundary_faces.insert(*face, elements[0]);
        }
    }

    boundary_faces
}

/// Group boundary faces by element block and create SurfaceMesh for each
fn group_by_block(
    mesh: &Mesh,
    boundary_faces: &HashMap<QuadFace, usize>,
    _face_adjacency: &HashMap<QuadFace, Vec<usize>>,
) -> Result<Vec<SurfaceMesh>> {
    // Create a map from element index to block name
    let mut elem_to_block: HashMap<usize, String> = HashMap::new();
    for (block_name, elem_indices) in &mesh.element_blocks {
        for &elem_idx in elem_indices {
            elem_to_block.insert(elem_idx, block_name.clone());
        }
    }

    // Group faces by block
    let mut block_faces: HashMap<String, Vec<QuadFace>> = HashMap::new();
    for (face, elem_idx) in boundary_faces {
        let block_name = elem_to_block
            .get(elem_idx)
            .ok_or_else(|| {
                ContactDetectorError::InvalidMeshTopology(format!(
                    "Element {} not found in any block",
                    elem_idx
                ))
            })?
            .clone();

        block_faces
            .entry(block_name)
            .or_default()
            .push(*face);
    }

    // Build SurfaceMesh for each block, further subdividing by connectivity and coplanarity
    let mut surfaces = Vec::new();
    for (block_name, faces) in block_faces {
        log::info!(
            "Subdividing block '{}' with {} faces into surface patches",
            block_name,
            faces.len()
        );

        // Subdivide faces into coplanar surface patches
        let surface_patches = subdivide_into_surface_patches(&faces, &mesh.nodes, &block_name)?;

        log::info!(
            "Block '{}' subdivided into {} surface patches",
            block_name,
            surface_patches.len()
        );

        surfaces.extend(surface_patches);
    }

    Ok(surfaces)
}

/// Subdivide a set of boundary faces into surface patches based on connectivity and coplanarity
fn subdivide_into_surface_patches(
    faces: &[QuadFace],
    nodes: &[Point],
    block_name: &str,
) -> Result<Vec<SurfaceMesh>> {
    use std::collections::{HashSet, VecDeque};

    if faces.is_empty() {
        return Ok(Vec::new());
    }

    // Maximum angle (in degrees) between normals to be considered coplanar
    const MAX_COPLANAR_ANGLE: f64 = 10.0;

    // Build face adjacency graph (which faces share edges)
    let face_adjacency = build_boundary_face_adjacency(faces);

    // Compute normals for all faces (needed for coplanarity check)
    let face_normals: Vec<_> = faces
        .iter()
        .map(|face| compute_face_normal(face, nodes))
        .collect::<Result<Vec<_>>>()?;

    // Group faces by connectivity and coplanarity using BFS
    let mut visited = HashSet::new();
    let mut surface_patches = Vec::new();

    for (seed_idx, _seed_face) in faces.iter().enumerate() {
        if visited.contains(&seed_idx) {
            continue;
        }

        // Start a new surface patch with this seed face
        let mut patch_faces = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(seed_idx);
        visited.insert(seed_idx);

        let seed_normal = &face_normals[seed_idx];

        // BFS to find all connected and coplanar faces
        while let Some(current_idx) = queue.pop_front() {
            patch_faces.push(faces[current_idx]);

            // Check all adjacent faces
            if let Some(adjacent_indices) = face_adjacency.get(&current_idx) {
                for &adj_idx in adjacent_indices {
                    if visited.contains(&adj_idx) {
                        continue;
                    }

                    // Check if adjacent face is coplanar with seed
                    let adj_normal = &face_normals[adj_idx];
                    let angle = crate::mesh::geometry::angle_between_vectors(seed_normal, adj_normal);

                    if angle <= MAX_COPLANAR_ANGLE {
                        visited.insert(adj_idx);
                        queue.push_back(adj_idx);
                    }
                }
            }
        }

        // Create a surface mesh for this patch
        let patch_name = format!("{}:patch_{}", block_name, surface_patches.len());
        let surface = build_surface_mesh(patch_name, patch_faces, nodes)?;
        surface_patches.push(surface);
    }

    Ok(surface_patches)
}

/// Build adjacency graph for boundary faces (which faces share edges)
fn build_boundary_face_adjacency(faces: &[QuadFace]) -> HashMap<usize, Vec<usize>> {
    use std::collections::HashMap;

    // Map from edge (as canonical pair of node IDs) to face indices
    let mut edge_to_faces: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

    for (face_idx, face) in faces.iter().enumerate() {
        // Get all 4 edges of the quad face
        let edges = [
            (face.node_ids[0], face.node_ids[1]),
            (face.node_ids[1], face.node_ids[2]),
            (face.node_ids[2], face.node_ids[3]),
            (face.node_ids[3], face.node_ids[0]),
        ];

        for (n1, n2) in edges {
            // Use canonical form (smaller node first)
            let edge = if n1 < n2 { (n1, n2) } else { (n2, n1) };
            edge_to_faces.entry(edge).or_default().push(face_idx);
        }
    }

    // Build adjacency map: face_idx -> list of adjacent face indices
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();

    for face_indices in edge_to_faces.values() {
        if face_indices.len() == 2 {
            // Two faces share this edge - they are adjacent
            let face_a = face_indices[0];
            let face_b = face_indices[1];

            adjacency.entry(face_a).or_default().push(face_b);
            adjacency.entry(face_b).or_default().push(face_a);
        }
    }

    adjacency
}

/// Build a SurfaceMesh from faces and nodes
fn build_surface_mesh(
    part_name: String,
    faces: Vec<QuadFace>,
    nodes: &[Point],
) -> Result<SurfaceMesh> {
    // Threshold for parallelization (below this, overhead isn't worth it)
    const PARALLEL_THRESHOLD: usize = 5000;

    // Compute geometric properties for each face (parallelized for large datasets)
    #[cfg(feature = "parallel")]
    let geometric_props: Result<Vec<_>> = if faces.len() >= PARALLEL_THRESHOLD {
        faces
            .par_iter()
            .map(|face| {
                let normal = compute_face_normal(face, nodes)?;
                let centroid = compute_face_centroid(face, nodes)?;
                let area = compute_face_area(face, nodes)?;
                Ok((normal, centroid, area))
            })
            .collect()
    } else {
        faces
            .iter()
            .map(|face| {
                let normal = compute_face_normal(face, nodes)?;
                let centroid = compute_face_centroid(face, nodes)?;
                let area = compute_face_area(face, nodes)?;
                Ok((normal, centroid, area))
            })
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let geometric_props: Result<Vec<_>> = faces
        .iter()
        .map(|face| {
            let normal = compute_face_normal(face, nodes)?;
            let centroid = compute_face_centroid(face, nodes)?;
            let area = compute_face_area(face, nodes)?;
            Ok((normal, centroid, area))
        })
        .collect();

    let props = geometric_props?;

    // Unzip the results into separate vectors
    let mut face_normals = Vec::with_capacity(props.len());
    let mut face_centroids = Vec::with_capacity(props.len());
    let mut face_areas = Vec::with_capacity(props.len());

    for (normal, centroid, area) in props {
        face_normals.push(normal);
        face_centroids.push(centroid);
        face_areas.push(area);
    }

    // Clone nodes for the surface mesh
    // Note: This could be optimized to only include nodes used by surface faces
    let surface_nodes = nodes.to_vec();

    let surface = SurfaceMesh {
        part_name,
        faces,
        face_normals,
        face_centroids,
        face_areas,
        nodes: surface_nodes,
    };

    Ok(surface)
}

/// Validate that the surface is closed (optional debugging aid)
/// A closed surface should have all edges shared by exactly 2 faces
pub fn validate_surface_closure(surface: &SurfaceMesh) -> Result<bool> {
    let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();

    for face in &surface.faces {
        // Get all 4 edges of the quad face
        let edges = [
            (face.node_ids[0], face.node_ids[1]),
            (face.node_ids[1], face.node_ids[2]),
            (face.node_ids[2], face.node_ids[3]),
            (face.node_ids[3], face.node_ids[0]),
        ];

        for (n1, n2) in edges {
            // Use canonical form (smaller node first) for consistent edge representation
            let edge = if n1 < n2 { (n1, n2) } else { (n2, n1) };
            *edge_count.entry(edge).or_insert(0) += 1;
        }
    }

    // Check if all edges are shared by exactly 2 faces
    let is_closed = edge_count.values().all(|&count| count == 2);

    if !is_closed {
        log::warn!(
            "Surface '{}' is not closed - some edges are not shared by exactly 2 faces",
            surface.part_name
        );
    }

    Ok(is_closed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::types::HexElement;

    fn make_single_hex_mesh() -> Mesh {
        // Create a simple 1x1x1 cube
        let nodes = vec![
            Point::new(0.0, 0.0, 0.0), // 0
            Point::new(1.0, 0.0, 0.0), // 1
            Point::new(1.0, 1.0, 0.0), // 2
            Point::new(0.0, 1.0, 0.0), // 3
            Point::new(0.0, 0.0, 1.0), // 4
            Point::new(1.0, 0.0, 1.0), // 5
            Point::new(1.0, 1.0, 1.0), // 6
            Point::new(0.0, 1.0, 1.0), // 7
        ];

        let element = HexElement::new([0, 1, 2, 3, 4, 5, 6, 7]);

        let mut element_blocks = HashMap::new();
        element_blocks.insert("Block1".to_string(), vec![0]);

        Mesh {
            nodes,
            elements: vec![element],
            element_blocks,
            node_sets: HashMap::new(),
            side_sets: HashMap::new(),
        }
    }

    #[test]
    fn test_single_hex_surface_extraction() {
        let mesh = make_single_hex_mesh();
        let surfaces = extract_surface(&mesh).unwrap();

        // With surface patch subdivision, each face of the hex becomes a separate surface
        // since they are perpendicular to each other (not coplanar)
        assert_eq!(surfaces.len(), 6);

        // Each surface patch should have 1 face
        for surface in &surfaces {
            assert_eq!(surface.faces.len(), 1);
            assert!(surface.part_name.starts_with("Block1:patch_"));
        }

        // Total faces should still be 6
        let total_faces: usize = surfaces.iter().map(|s| s.faces.len()).sum();
        assert_eq!(total_faces, 6);
    }

    #[test]
    fn test_face_adjacency() {
        let mesh = make_single_hex_mesh();
        let adjacency = build_face_adjacency(&mesh).unwrap();

        // Single hex has 6 unique faces, each with 1 adjacent element
        assert_eq!(adjacency.len(), 6);
        for (_, elements) in &adjacency {
            assert_eq!(elements.len(), 1);
        }
    }

    #[test]
    fn test_boundary_faces() {
        let mesh = make_single_hex_mesh();
        let adjacency = build_face_adjacency(&mesh).unwrap();
        let boundary = extract_boundary_faces(&adjacency);

        // All 6 faces should be boundary faces for a single hex
        assert_eq!(boundary.len(), 6);
    }

    #[test]
    fn test_two_hex_shared_face() {
        // Create two hexes sharing a face
        let nodes = vec![
            // First hex
            Point::new(0.0, 0.0, 0.0), // 0
            Point::new(1.0, 0.0, 0.0), // 1
            Point::new(1.0, 1.0, 0.0), // 2
            Point::new(0.0, 1.0, 0.0), // 3
            Point::new(0.0, 0.0, 1.0), // 4
            Point::new(1.0, 0.0, 1.0), // 5
            Point::new(1.0, 1.0, 1.0), // 6
            Point::new(0.0, 1.0, 1.0), // 7
            // Second hex (sharing top face with first)
            Point::new(0.0, 0.0, 2.0), // 8
            Point::new(1.0, 0.0, 2.0), // 9
            Point::new(1.0, 1.0, 2.0), // 10
            Point::new(0.0, 1.0, 2.0), // 11
        ];

        let hex1 = HexElement::new([0, 1, 2, 3, 4, 5, 6, 7]);
        let hex2 = HexElement::new([4, 5, 6, 7, 8, 9, 10, 11]);

        let mut element_blocks = HashMap::new();
        element_blocks.insert("Block1".to_string(), vec![0, 1]);

        let mesh = Mesh {
            nodes,
            elements: vec![hex1, hex2],
            element_blocks,
            node_sets: HashMap::new(),
            side_sets: HashMap::new(),
        };

        let adjacency = build_face_adjacency(&mesh).unwrap();
        let boundary = extract_boundary_faces(&adjacency);

        // Two hexes share 1 face, so total boundary should be:
        // 12 total faces - 2 shared = 10 boundary faces
        assert_eq!(boundary.len(), 10);
    }
}
