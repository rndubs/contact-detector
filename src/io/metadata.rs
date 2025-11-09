//! JSON metadata export for contact detection results

use crate::contact::{ContactCriteria, ContactResults, SurfaceMetrics};
use crate::error::Result;
use crate::mesh::SurfaceMesh;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complete metadata export for contact detection analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct ContactMetadata {
    /// Source mesh file
    pub mesh_file: String,

    /// Timestamp when analysis was performed
    pub timestamp: String,

    /// Detection criteria used
    pub detection_criteria: DetectionCriteriaJson,

    /// All detected contact pairs
    pub contact_pairs: Vec<ContactPairMetadata>,
}

/// JSON representation of detection criteria
#[derive(Debug, Serialize, Deserialize)]
pub struct DetectionCriteriaJson {
    pub max_gap: f64,
    pub max_penetration: f64,
    pub max_angle: f64,
    pub min_pairs: usize,
}

/// Metadata for a single contact pair
#[derive(Debug, Serialize, Deserialize)]
pub struct ContactPairMetadata {
    pub pair_id: usize,
    pub surface_a: SurfaceInfo,
    pub surface_b: SurfaceInfo,
    pub contact_statistics: ContactStatistics,
}

/// Information about a single surface in a contact pair
#[derive(Debug, Serialize, Deserialize)]
pub struct SurfaceInfo {
    pub name: String,
    pub sideset_name: String,
    pub block_id: Option<usize>,
    pub patch_id: Option<usize>,
    pub total_faces: usize,
    pub paired_faces: usize,
    pub unpaired_faces: usize,
    pub total_area: f64,
    pub paired_area: f64,
    pub avg_normal: [f64; 3],
}

/// Contact statistics for a pair
#[derive(Debug, Serialize, Deserialize)]
pub struct ContactStatistics {
    pub num_pairs: usize,
    pub avg_distance: f64,
    pub min_distance: f64,
    pub max_distance: f64,
    pub std_dev_distance: f64,
    pub avg_normal_angle: f64,
    pub normal_alignment: String,
}

impl ContactMetadata {
    /// Create new contact metadata from analysis results
    pub fn new(
        mesh_file: String,
        criteria: &ContactCriteria,
        min_pairs: usize,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();

        Self {
            mesh_file,
            timestamp,
            detection_criteria: DetectionCriteriaJson {
                max_gap: criteria.max_gap_distance,
                max_penetration: criteria.max_penetration,
                max_angle: criteria.max_normal_angle,
                min_pairs,
            },
            contact_pairs: Vec::new(),
        }
    }

    /// Add a contact pair to the metadata
    pub fn add_contact_pair(
        &mut self,
        pair_id: usize,
        surface_a: &SurfaceMesh,
        surface_b: &SurfaceMesh,
        results: &ContactResults,
        metrics_a: &SurfaceMetrics,
        metrics_b: &SurfaceMetrics,
    ) {
        // Compute average normals for each surface
        let avg_normal_a = compute_average_normal(surface_a);
        let avg_normal_b = compute_average_normal(surface_b);

        // Parse block and patch IDs from surface names if available
        let (block_a, patch_a) = parse_surface_name(&surface_a.part_name);
        let (block_b, patch_b) = parse_surface_name(&surface_b.part_name);

        // Generate sideset names
        let sideset_a = format!("auto_contact_{}", sanitize_name(&surface_a.part_name));
        let sideset_b = format!("auto_contact_{}", sanitize_name(&surface_b.part_name));

        // Determine normal alignment
        let normal_alignment = if metrics_a.avg_normal_angle > 150.0 {
            "opposed".to_string()
        } else if metrics_a.avg_normal_angle < 30.0 {
            "aligned".to_string()
        } else {
            "angled".to_string()
        };

        let pair_metadata = ContactPairMetadata {
            pair_id,
            surface_a: SurfaceInfo {
                name: surface_a.part_name.clone(),
                sideset_name: sideset_a,
                block_id: block_a,
                patch_id: patch_a,
                total_faces: surface_a.faces.len(),
                paired_faces: metrics_a.num_pairs,
                unpaired_faces: metrics_a.num_unpaired,
                total_area: metrics_a.total_area,
                paired_area: metrics_a.paired_area,
                avg_normal: avg_normal_a,
            },
            surface_b: SurfaceInfo {
                name: surface_b.part_name.clone(),
                sideset_name: sideset_b,
                block_id: block_b,
                patch_id: patch_b,
                total_faces: surface_b.faces.len(),
                paired_faces: metrics_b.num_pairs,
                unpaired_faces: metrics_b.num_unpaired,
                total_area: metrics_b.total_area,
                paired_area: metrics_b.paired_area,
                avg_normal: avg_normal_b,
            },
            contact_statistics: ContactStatistics {
                num_pairs: results.num_pairs(),
                avg_distance: metrics_a.avg_distance,
                min_distance: metrics_a.min_distance,
                max_distance: metrics_a.max_distance,
                std_dev_distance: metrics_a.std_dev_distance,
                avg_normal_angle: metrics_a.avg_normal_angle,
                normal_alignment,
            },
        };

        self.contact_pairs.push(pair_metadata);
    }

    /// Export metadata to JSON file
    pub fn export<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = std::fs::File::create(path.as_ref())?;
        serde_json::to_writer_pretty(file, self).map_err(|e| {
            crate::error::ContactDetectorError::ConfigError(format!("Failed to write JSON metadata: {}", e))
        })?;
        Ok(())
    }
}

/// Compute the average normal vector for a surface
fn compute_average_normal(surface: &SurfaceMesh) -> [f64; 3] {
    if surface.face_normals.is_empty() {
        return [0.0, 0.0, 0.0];
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_z = 0.0;

    for normal in &surface.face_normals {
        sum_x += normal.x;
        sum_y += normal.y;
        sum_z += normal.z;
    }

    let count = surface.face_normals.len() as f64;
    let avg_x = sum_x / count;
    let avg_y = sum_y / count;
    let avg_z = sum_z / count;

    // Normalize
    let magnitude = (avg_x * avg_x + avg_y * avg_y + avg_z * avg_z).sqrt();

    if magnitude > 1e-10 {
        [avg_x / magnitude, avg_y / magnitude, avg_z / magnitude]
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// Parse block and patch IDs from surface name (e.g., "Block_1:patch_4" -> (Some(1), Some(4)))
fn parse_surface_name(name: &str) -> (Option<usize>, Option<usize>) {
    let mut block_id = None;
    let mut patch_id = None;

    // Try to extract block ID
    if let Some(block_part) = name.split(':').next() {
        if let Some(num_str) = block_part.split('_').last() {
            if let Ok(id) = num_str.parse::<usize>() {
                block_id = Some(id);
            }
        }
    }

    // Try to extract patch ID
    if let Some(patch_part) = name.split(':').nth(1) {
        if let Some(num_str) = patch_part.split('_').last() {
            if let Ok(id) = num_str.parse::<usize>() {
                patch_id = Some(id);
            }
        }
    }

    (block_id, patch_id)
}

/// Sanitize a name for use in sideset names
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_surface_name() {
        assert_eq!(parse_surface_name("Block_1:patch_4"), (Some(1), Some(4)));
        assert_eq!(parse_surface_name("Block_2:patch_1"), (Some(2), Some(1)));
        assert_eq!(parse_surface_name("SimpleBlock"), (None, None));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Block_1:patch_4"), "Block_1_patch_4");
        assert_eq!(sanitize_name("Part-A/B"), "Part_A_B");
    }
}
