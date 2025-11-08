//! Geometric operations for mesh elements

use crate::error::{ContactDetectorError, Result};
use crate::mesh::types::{Point, QuadFace, Vec3};

/// Compute the normal vector of a quad face
/// Uses the cross product of diagonals to get a normal pointing outward
pub fn compute_face_normal(face: &QuadFace, nodes: &[Point]) -> Result<Vec3> {
    let n0 = get_node(nodes, face.node_ids[0])?;
    let n1 = get_node(nodes, face.node_ids[1])?;
    let n2 = get_node(nodes, face.node_ids[2])?;
    let n3 = get_node(nodes, face.node_ids[3])?;

    // Compute vectors along edges
    let v1 = n2 - n0; // diagonal 1
    let v2 = n3 - n1; // diagonal 2

    // Cross product gives normal
    let normal = v1.cross(&v2);

    // Normalize
    let norm = normal.norm();
    if norm < 1e-12 {
        return Err(ContactDetectorError::GeometryError(
            "Degenerate face (zero normal)".to_string(),
        ));
    }

    Ok(normal / norm)
}

/// Compute the centroid of a quad face
pub fn compute_face_centroid(face: &QuadFace, nodes: &[Point]) -> Result<Point> {
    let n0 = get_node(nodes, face.node_ids[0])?;
    let n1 = get_node(nodes, face.node_ids[1])?;
    let n2 = get_node(nodes, face.node_ids[2])?;
    let n3 = get_node(nodes, face.node_ids[3])?;

    // Average of all four nodes
    let centroid = (n0.coords + n1.coords + n2.coords + n3.coords) / 4.0;

    Ok(Point::from(centroid))
}

/// Compute the area of a quad face
/// Uses the cross product of diagonals divided by 2
pub fn compute_face_area(face: &QuadFace, nodes: &[Point]) -> Result<f64> {
    let n0 = get_node(nodes, face.node_ids[0])?;
    let n1 = get_node(nodes, face.node_ids[1])?;
    let n2 = get_node(nodes, face.node_ids[2])?;
    let n3 = get_node(nodes, face.node_ids[3])?;

    // For a quad, area = |diagonal1 × diagonal2| / 2
    let d1 = n2 - n0;
    let d2 = n3 - n1;

    let cross = d1.cross(&d2);
    let area = cross.norm() / 2.0;

    if area < 1e-12 {
        return Err(ContactDetectorError::GeometryError(
            "Degenerate face (zero area)".to_string(),
        ));
    }

    Ok(area)
}

/// Compute the distance between two points
pub fn distance(p1: &Point, p2: &Point) -> f64 {
    (p2 - p1).norm()
}

/// Compute the signed distance from a point to a plane defined by a point and normal
/// Positive distance means the point is on the side the normal points to
pub fn signed_distance_to_plane(point: &Point, plane_point: &Point, plane_normal: &Vec3) -> f64 {
    let v = point - plane_point;
    v.dot(plane_normal)
}

/// Project a point onto a plane defined by a point and normal
pub fn project_point_to_plane(point: &Point, plane_point: &Point, plane_normal: &Vec3) -> Point {
    let dist = signed_distance_to_plane(point, plane_point, plane_normal);
    Point::from(point.coords - dist * plane_normal)
}

/// Compute the angle between two vectors in degrees
pub fn angle_between_vectors(v1: &Vec3, v2: &Vec3) -> f64 {
    let dot = v1.dot(v2);
    let norm_product = v1.norm() * v2.norm();

    if norm_product < 1e-12 {
        return 0.0;
    }

    let cos_angle = (dot / norm_product).clamp(-1.0, 1.0);
    cos_angle.acos().to_degrees()
}

/// Helper to safely get a node from the node array
fn get_node(nodes: &[Point], index: usize) -> Result<&Point> {
    nodes.get(index).ok_or_else(|| {
        ContactDetectorError::InvalidMeshTopology(format!("Node index {} out of bounds", index))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn make_square_face() -> (QuadFace, Vec<Point>) {
        let face = QuadFace::new([0, 1, 2, 3]);
        let nodes = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];
        (face, nodes)
    }

    #[test]
    fn test_face_normal() {
        let (face, nodes) = make_square_face();
        let normal = compute_face_normal(&face, &nodes).unwrap();

        // Normal should point in +z direction
        assert_relative_eq!(normal.x, 0.0, epsilon = 1e-10);
        assert_relative_eq!(normal.y, 0.0, epsilon = 1e-10);
        assert_relative_eq!(normal.z.abs(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_face_centroid() {
        let (face, nodes) = make_square_face();
        let centroid = compute_face_centroid(&face, &nodes).unwrap();

        // Centroid should be at (0.5, 0.5, 0.0)
        assert_relative_eq!(centroid.x, 0.5, epsilon = 1e-10);
        assert_relative_eq!(centroid.y, 0.5, epsilon = 1e-10);
        assert_relative_eq!(centroid.z, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_face_area() {
        let (face, nodes) = make_square_face();
        let area = compute_face_area(&face, &nodes).unwrap();

        // Area should be 1.0 (1x1 square)
        assert_relative_eq!(area, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_distance() {
        let p1 = Point::new(0.0, 0.0, 0.0);
        let p2 = Point::new(3.0, 4.0, 0.0);

        let d = distance(&p1, &p2);
        assert_relative_eq!(d, 5.0, epsilon = 1e-10); // 3-4-5 triangle
    }

    #[test]
    fn test_signed_distance_to_plane() {
        let plane_point = Point::new(0.0, 0.0, 0.0);
        let plane_normal = Vec3::new(0.0, 0.0, 1.0);

        let point_above = Point::new(0.0, 0.0, 2.0);
        let point_below = Point::new(0.0, 0.0, -1.5);

        assert_relative_eq!(
            signed_distance_to_plane(&point_above, &plane_point, &plane_normal),
            2.0,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            signed_distance_to_plane(&point_below, &plane_point, &plane_normal),
            -1.5,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_angle_between_vectors() {
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);
        let v3 = Vec3::new(-1.0, 0.0, 0.0);

        assert_relative_eq!(angle_between_vectors(&v1, &v2), 90.0, epsilon = 1e-8);
        assert_relative_eq!(angle_between_vectors(&v1, &v3), 180.0, epsilon = 1e-8);
        assert_relative_eq!(angle_between_vectors(&v1, &v1), 0.0, epsilon = 1e-8);
    }
}
