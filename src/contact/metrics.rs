//! Surface-level and element-level metric computation

use crate::contact::types::ContactResults;
use crate::mesh::types::SurfaceMesh;
use serde::{Deserialize, Serialize};

/// Surface-level contact metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceMetrics {
    /// Total surface area
    pub total_area: f64,

    /// Area of paired faces
    pub paired_area: f64,

    /// Area of unpaired faces
    pub unpaired_area: f64,

    /// Average distance (area-weighted)
    pub avg_distance: f64,

    /// Standard deviation of distance
    pub std_dev_distance: f64,

    /// Minimum distance
    pub min_distance: f64,

    /// Maximum distance
    pub max_distance: f64,

    /// Average normal angle
    pub avg_normal_angle: f64,

    /// Number of contact pairs
    pub num_pairs: usize,

    /// Number of unpaired faces
    pub num_unpaired: usize,
}

impl SurfaceMetrics {
    /// Compute surface metrics from contact results and surface mesh
    pub fn compute(results: &ContactResults, surface: &SurfaceMesh) -> Self {
        let total_area: f64 = surface.face_areas.iter().sum();

        let mut paired_area = 0.0;
        let mut weighted_distance_sum = 0.0;
        let mut angle_sum = 0.0;
        let mut min_dist = f64::MAX;
        let mut max_dist = f64::MIN;

        // Compute paired area and statistics
        for pair in &results.pairs {
            let face_area = surface.face_areas[pair.surface_a_face_id];
            paired_area += face_area;
            weighted_distance_sum += pair.distance * face_area;
            angle_sum += pair.normal_angle;

            min_dist = min_dist.min(pair.distance);
            max_dist = max_dist.max(pair.distance);
        }

        let num_pairs = results.pairs.len();
        let avg_distance = if num_pairs > 0 {
            weighted_distance_sum / paired_area
        } else {
            0.0
        };

        let avg_normal_angle = if num_pairs > 0 {
            angle_sum / num_pairs as f64
        } else {
            0.0
        };

        // Compute standard deviation
        let mut variance_sum = 0.0;
        for pair in &results.pairs {
            let diff = pair.distance - avg_distance;
            let face_area = surface.face_areas[pair.surface_a_face_id];
            variance_sum += diff * diff * face_area;
        }

        let std_dev_distance = if paired_area > 0.0 {
            (variance_sum / paired_area).sqrt()
        } else {
            0.0
        };

        let unpaired_area = total_area - paired_area;

        Self {
            total_area,
            paired_area,
            unpaired_area,
            avg_distance,
            std_dev_distance,
            min_distance: if num_pairs > 0 { min_dist } else { 0.0 },
            max_distance: if num_pairs > 0 { max_dist } else { 0.0 },
            avg_normal_angle,
            num_pairs,
            num_unpaired: results.unpaired_a.len(),
        }
    }

    /// Print metrics summary
    pub fn print_summary(&self, surface_name: &str) {
        println!("\n{}", "=".repeat(60));
        println!("SURFACE METRICS: {}", surface_name);
        println!("{}", "=".repeat(60));
        println!();
        println!("  Total Area:      {:.6}", self.total_area);
        println!("  Paired Area:     {:.6}  ({:.1}%)", self.paired_area, self.paired_area / self.total_area * 100.0);
        println!("  Unpaired Area:   {:.6}  ({:.1}%)", self.unpaired_area, self.unpaired_area / self.total_area * 100.0);
        println!();
        println!("  Contact Pairs:   {}", self.num_pairs);
        println!("  Unpaired Faces:  {}", self.num_unpaired);
        println!();

        if self.num_pairs > 0 {
            println!("  Distance Statistics (area-weighted):");
            println!("    Average:   {:.6}", self.avg_distance);
            println!("    Std Dev:   {:.6}", self.std_dev_distance);
            println!("    Min:       {:.6}", self.min_distance);
            println!("    Max:       {:.6}", self.max_distance);
            println!();
            println!("  Normal Angle:");
            println!("    Average:   {:.2}°", self.avg_normal_angle);
            println!();
        }

        println!("{}", "=".repeat(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::types::{ContactCriteria, ContactPair};
    use crate::mesh::types::{Point, QuadFace, Vec3};

    fn make_test_data() -> (ContactResults, SurfaceMesh) {
        let surface = SurfaceMesh {
            part_name: "TestSurface".to_string(),
            faces: vec![
                QuadFace::new([0, 1, 2, 3]),
                QuadFace::new([4, 5, 6, 7]),
            ],
            face_normals: vec![Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 1.0)],
            face_centroids: vec![Point::new(0.5, 0.5, 0.0), Point::new(1.5, 0.5, 0.0)],
            face_areas: vec![1.0, 2.0],
            nodes: vec![],
        };

        let mut results = ContactResults::new(
            "SurfaceA".to_string(),
            "SurfaceB".to_string(),
            ContactCriteria::default(),
        );

        results.pairs.push(ContactPair {
            surface_a_face_id: 0,
            surface_b_face_id: 0,
            distance: 0.001,
            normal_angle: 10.0,
            contact_point: Point::new(0.5, 0.5, 0.0),
        });

        results.pairs.push(ContactPair {
            surface_a_face_id: 1,
            surface_b_face_id: 1,
            distance: 0.002,
            normal_angle: 20.0,
            contact_point: Point::new(1.5, 0.5, 0.0),
        });

        (results, surface)
    }

    #[test]
    fn test_surface_metrics_computation() {
        let (results, surface) = make_test_data();
        let metrics = SurfaceMetrics::compute(&results, &surface);

        assert_eq!(metrics.total_area, 3.0);
        assert_eq!(metrics.paired_area, 3.0);
        assert_eq!(metrics.unpaired_area, 0.0);
        assert_eq!(metrics.num_pairs, 2);
        assert_eq!(metrics.num_unpaired, 0);

        // Area-weighted average: (1.0 * 0.001 + 2.0 * 0.002) / 3.0
        let expected_avg = (1.0 * 0.001 + 2.0 * 0.002) / 3.0;
        assert!((metrics.avg_distance - expected_avg).abs() < 1e-10);

        assert_eq!(metrics.min_distance, 0.001);
        assert_eq!(metrics.max_distance, 0.002);

        // Simple average of angles
        assert_eq!(metrics.avg_normal_angle, 15.0);
    }
}
