//! Contact detection data types

use crate::mesh::types::Point;
use serde::{Deserialize, Serialize};

/// Contact pair between two surface faces
#[derive(Debug, Clone)]
pub struct ContactPair {
    /// Surface A face index
    pub surface_a_face_id: usize,

    /// Surface B face index
    pub surface_b_face_id: usize,

    /// Signed distance between faces (+ for gap, - for overlap)
    pub distance: f64,

    /// Angle between face normals in degrees
    pub normal_angle: f64,

    /// Contact point on surface B
    pub contact_point: Point,
}

/// Criteria for contact detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactCriteria {
    /// Maximum gap distance to consider as contact
    pub max_gap_distance: f64,

    /// Maximum penetration distance (overlap)
    pub max_penetration: f64,

    /// Maximum normal angle in degrees
    pub max_normal_angle: f64,

    /// Search radius multiplier for spatial queries
    pub search_radius_multiplier: f64,
}

impl Default for ContactCriteria {
    fn default() -> Self {
        Self {
            max_gap_distance: 0.005,
            max_penetration: 0.001,
            max_normal_angle: 45.0,
            search_radius_multiplier: 2.0,
        }
    }
}

impl ContactCriteria {
    /// Create new contact criteria
    pub fn new(max_gap: f64, max_penetration: f64, max_angle: f64) -> Self {
        Self {
            max_gap_distance: max_gap,
            max_penetration,
            max_normal_angle: max_angle,
            search_radius_multiplier: 2.0,
        }
    }

    /// Get the search radius for spatial queries
    pub fn search_radius(&self) -> f64 {
        self.max_gap_distance * self.search_radius_multiplier
    }

    /// Check if a distance is within contact range
    pub fn is_in_range(&self, distance: f64) -> bool {
        distance >= -self.max_penetration && distance <= self.max_gap_distance
    }

    /// Check if a normal angle is within tolerance
    pub fn is_angle_valid(&self, angle: f64) -> bool {
        angle <= self.max_normal_angle
    }
}

/// Results from contact detection
#[derive(Debug, Clone)]
pub struct ContactResults {
    /// Name of surface A
    pub surface_a_name: String,

    /// Name of surface B
    pub surface_b_name: String,

    /// Contact pairs found
    pub pairs: Vec<ContactPair>,

    /// Face indices on surface A that have no contact pair
    pub unpaired_a: Vec<usize>,

    /// Face indices on surface B that have no contact pair
    pub unpaired_b: Vec<usize>,

    /// Criteria used for detection
    pub criteria: ContactCriteria,
}

impl ContactResults {
    /// Create new contact results
    pub fn new(
        surface_a_name: String,
        surface_b_name: String,
        criteria: ContactCriteria,
    ) -> Self {
        Self {
            surface_a_name,
            surface_b_name,
            pairs: Vec::new(),
            unpaired_a: Vec::new(),
            unpaired_b: Vec::new(),
            criteria,
        }
    }

    /// Get number of contact pairs
    pub fn num_pairs(&self) -> usize {
        self.pairs.len()
    }

    /// Get average distance
    pub fn avg_distance(&self) -> f64 {
        if self.pairs.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.pairs.iter().map(|p| p.distance).sum();
        sum / self.pairs.len() as f64
    }

    /// Get minimum distance
    pub fn min_distance(&self) -> f64 {
        self.pairs
            .iter()
            .map(|p| p.distance)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }

    /// Get maximum distance
    pub fn max_distance(&self) -> f64 {
        self.pairs
            .iter()
            .map(|p| p.distance)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }

    /// Get average normal angle
    pub fn avg_normal_angle(&self) -> f64 {
        if self.pairs.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.pairs.iter().map(|p| p.normal_angle).sum();
        sum / self.pairs.len() as f64
    }

    /// Print summary statistics
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("CONTACT DETECTION RESULTS");
        println!("{}", "=".repeat(60));
        println!();
        println!("  Surface A: {}", self.surface_a_name);
        println!("  Surface B: {}", self.surface_b_name);
        println!();
        println!("  Contact Pairs: {}", self.num_pairs());
        println!("  Unpaired A:    {}", self.unpaired_a.len());
        println!("  Unpaired B:    {}", self.unpaired_b.len());
        println!();

        if !self.pairs.is_empty() {
            println!("  Distance Statistics:");
            println!("    Average: {:.6}", self.avg_distance());
            println!("    Min:     {:.6}", self.min_distance());
            println!("    Max:     {:.6}", self.max_distance());
            println!();
            println!("  Normal Angle Statistics:");
            println!("    Average: {:.2}°", self.avg_normal_angle());
            println!();
        }

        println!("  Criteria:");
        println!("    Max Gap:         {:.6}", self.criteria.max_gap_distance);
        println!("    Max Penetration: {:.6}", self.criteria.max_penetration);
        println!("    Max Angle:       {:.1}°", self.criteria.max_normal_angle);
        println!();
        println!("{}", "=".repeat(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_criteria_default() {
        let criteria = ContactCriteria::default();
        assert_eq!(criteria.max_gap_distance, 0.005);
        assert_eq!(criteria.max_penetration, 0.001);
        assert_eq!(criteria.max_normal_angle, 45.0);
    }

    #[test]
    fn test_contact_criteria_is_in_range() {
        let criteria = ContactCriteria::default();

        assert!(criteria.is_in_range(0.0)); // Zero distance is valid
        assert!(criteria.is_in_range(0.003)); // Small gap is valid
        assert!(criteria.is_in_range(-0.0005)); // Small overlap is valid
        assert!(!criteria.is_in_range(0.01)); // Large gap is invalid
        assert!(!criteria.is_in_range(-0.002)); // Large overlap is invalid
    }

    #[test]
    fn test_contact_criteria_is_angle_valid() {
        let criteria = ContactCriteria::default();

        assert!(criteria.is_angle_valid(0.0));
        assert!(criteria.is_angle_valid(30.0));
        assert!(criteria.is_angle_valid(45.0));
        assert!(!criteria.is_angle_valid(90.0));
    }
}
