//! Contact pair detection algorithm

use crate::contact::types::{ContactCriteria, ContactPair, ContactResults};
use crate::error::Result;
use crate::mesh::geometry::{
    angle_between_vectors, project_point_to_plane, signed_distance_to_plane,
};
use crate::mesh::types::SurfaceMesh;
use kiddo::ImmutableKdTree;
use std::collections::HashSet;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Detect contact pairs between two surfaces
pub fn detect_contact_pairs(
    surface_a: &SurfaceMesh,
    surface_b: &SurfaceMesh,
    criteria: &ContactCriteria,
) -> Result<ContactResults> {
    log::info!(
        "Detecting contact pairs between '{}' and '{}'",
        surface_a.part_name,
        surface_b.part_name
    );

    let mut results = ContactResults::new(
        surface_a.part_name.clone(),
        surface_b.part_name.clone(),
        criteria.clone(),
    );

    // Build spatial index for surface B
    log::info!("Building spatial index for surface B...");
    let tree_b = build_face_kdtree(surface_b);

    // For each face on surface A, find closest face on surface B (parallelized for large datasets)
    log::info!("Searching for contact pairs...");

    // Threshold for parallelization (below this, overhead isn't worth it)
    const PARALLEL_THRESHOLD: usize = 1000;

    #[cfg(feature = "parallel")]
    let face_results: Vec<_> = if surface_a.faces.len() >= PARALLEL_THRESHOLD {
        surface_a
            .faces
            .par_iter()
            .enumerate()
            .map(|(face_a_idx, _face_a)| {
                find_best_match(face_a_idx, surface_a, surface_b, &tree_b, criteria)
            })
            .collect()
    } else {
        surface_a
            .faces
            .iter()
            .enumerate()
            .map(|(face_a_idx, _face_a)| {
                find_best_match(face_a_idx, surface_a, surface_b, &tree_b, criteria)
            })
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let face_results: Vec<_> = surface_a
        .faces
        .iter()
        .enumerate()
        .map(|(face_a_idx, _face_a)| {
            find_best_match(face_a_idx, surface_a, surface_b, &tree_b, criteria)
        })
        .collect();

    // Collect results
    let mut paired_b = HashSet::new();
    for (face_a_idx, result) in face_results.into_iter().enumerate() {
        match result {
            Some(pair) => {
                paired_b.insert(pair.surface_b_face_id);
                results.pairs.push(pair);
            }
            None => {
                results.unpaired_a.push(face_a_idx);
            }
        }
    }

    // Find unpaired faces on B (parallelized for large datasets)
    #[cfg(feature = "parallel")]
    let unpaired_b: Vec<usize> = if surface_b.faces.len() >= PARALLEL_THRESHOLD {
        (0..surface_b.faces.len())
            .into_par_iter()
            .filter(|face_b_idx| !paired_b.contains(face_b_idx))
            .collect()
    } else {
        (0..surface_b.faces.len())
            .filter(|face_b_idx| !paired_b.contains(face_b_idx))
            .collect()
    };

    #[cfg(not(feature = "parallel"))]
    let unpaired_b: Vec<usize> = (0..surface_b.faces.len())
        .filter(|face_b_idx| !paired_b.contains(face_b_idx))
        .collect();

    results.unpaired_b = unpaired_b;

    log::info!(
        "Found {} contact pairs, {} unpaired on A, {} unpaired on B",
        results.num_pairs(),
        results.unpaired_a.len(),
        results.unpaired_b.len()
    );

    Ok(results)
}

/// Find the best matching face on surface B for a given face on surface A
fn find_best_match(
    face_a_idx: usize,
    surface_a: &SurfaceMesh,
    surface_b: &SurfaceMesh,
    tree_b: &ImmutableKdTree<f64, 3>,
    criteria: &ContactCriteria,
) -> Option<ContactPair> {
    let centroid_a = &surface_a.face_centroids[face_a_idx];
    let normal_a = &surface_a.face_normals[face_a_idx];

    // Query k-d tree for nearest faces on surface B
    let search_radius = criteria.search_radius();
    let nearest = tree_b.within::<kiddo::SquaredEuclidean>(
        &[centroid_a.x, centroid_a.y, centroid_a.z],
        search_radius * search_radius,
    );

    // Find best matching face on B
    let mut best_match: Option<ContactPair> = None;
    let mut best_distance_abs = f64::MAX;

    for neighbor in nearest.iter() {
        let face_b_idx = neighbor.item as usize;
        let centroid_b = &surface_b.face_centroids[face_b_idx];
        let normal_b = &surface_b.face_normals[face_b_idx];

        // Compute signed distance from A to B along A's normal
        let distance = signed_distance_to_plane(centroid_b, centroid_a, normal_a);

        // Check if distance is within range
        if !criteria.is_in_range(distance) {
            continue;
        }

        // Compute angle between normals
        let angle = angle_between_vectors(normal_a, normal_b);

        // Check if angle is within tolerance
        if !criteria.is_angle_valid(angle) {
            continue;
        }

        // Project centroid A onto B's plane to get contact point
        let contact_point = project_point_to_plane(centroid_a, centroid_b, normal_b);

        // Keep track of the best match (smallest absolute distance)
        let distance_abs = distance.abs();
        if distance_abs < best_distance_abs {
            best_distance_abs = distance_abs;
            best_match = Some(ContactPair {
                surface_a_face_id: face_a_idx,
                surface_b_face_id: face_b_idx,
                distance,
                normal_angle: angle,
                contact_point,
            });
        }
    }

    best_match
}

/// Build a k-d tree for spatial indexing of face centroids
fn build_face_kdtree(surface: &SurfaceMesh) -> ImmutableKdTree<f64, 3> {
    // Collect all points
    let points: Vec<[f64; 3]> = surface
        .face_centroids
        .iter()
        .map(|c| [c.x, c.y, c.z])
        .collect();

    // Build immutable k-d tree (indices are implicit: 0, 1, 2, ...)
    ImmutableKdTree::new_from_slice(&points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::types::{Point, QuadFace, Vec3};

    fn make_parallel_surfaces() -> (SurfaceMesh, SurfaceMesh) {
        // Surface A: flat square at z=0
        let nodes_a = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];

        let face_a = QuadFace::new([0, 1, 2, 3]);

        let surface_a = SurfaceMesh {
            part_name: "SurfaceA".to_string(),
            faces: vec![face_a],
            face_normals: vec![Vec3::new(0.0, 0.0, 1.0)],
            face_centroids: vec![Point::new(0.5, 0.5, 0.0)],
            face_areas: vec![1.0],
            nodes: nodes_a,
        };

        // Surface B: flat square at z=0.001 (small gap)
        let nodes_b = vec![
            Point::new(0.0, 0.0, 0.001),
            Point::new(1.0, 0.0, 0.001),
            Point::new(1.0, 1.0, 0.001),
            Point::new(0.0, 1.0, 0.001),
        ];

        let face_b = QuadFace::new([0, 1, 2, 3]);

        let surface_b = SurfaceMesh {
            part_name: "SurfaceB".to_string(),
            faces: vec![face_b],
            face_normals: vec![Vec3::new(0.0, 0.0, -1.0)], // Opposite normal
            face_centroids: vec![Point::new(0.5, 0.5, 0.001)],
            face_areas: vec![1.0],
            nodes: nodes_b,
        };

        (surface_a, surface_b)
    }

    #[test]
    fn test_detect_contact_pairs_parallel_surfaces() {
        let (surface_a, surface_b) = make_parallel_surfaces();
        // Use larger angle tolerance since surfaces have opposite normals (180 degrees)
        let criteria = ContactCriteria::new(0.005, 0.001, 180.0);

        let results = detect_contact_pairs(&surface_a, &surface_b, &criteria).unwrap();

        assert_eq!(results.num_pairs(), 1);
        assert_eq!(results.unpaired_a.len(), 0);
        assert_eq!(results.unpaired_b.len(), 0);

        let pair = &results.pairs[0];
        assert!((pair.distance - 0.001).abs() < 1e-6);
        assert!((pair.normal_angle - 180.0).abs() < 1.0); // Opposite normals
    }

    #[test]
    fn test_build_face_kdtree() {
        let (surface_a, _) = make_parallel_surfaces();
        let tree = build_face_kdtree(&surface_a);

        // Should have one entry
        let nearest = tree.nearest_n::<kiddo::SquaredEuclidean>(&[0.5, 0.5, 0.0], 1);

        assert_eq!(nearest.len(), 1);
        assert_eq!(nearest[0].item, 0); // Face index should be 0
    }
}
