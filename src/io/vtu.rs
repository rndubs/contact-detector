//! VTU (VTK Unstructured Grid) file writer

use crate::error::{ContactDetectorError, Result};
use crate::mesh::types::{Mesh, SurfaceMesh};
use std::path::Path;
use vtkio::model::*;

/// Default VTK file format version (2.2 for broad compatibility)
/// This version is compatible with ParaView 6.0.1 and most VTK-based tools
pub const DEFAULT_VTK_VERSION: (u8, u8) = (2, 2);

/// Write a surface mesh to a VTU file
pub fn write_surface_to_vtu(
    surface: &SurfaceMesh,
    output_path: &Path,
    vtk_version: Option<(u8, u8)>,
) -> Result<()> {
    let version = vtk_version.unwrap_or(DEFAULT_VTK_VERSION);
    log::info!(
        "Writing surface '{}' with {} faces to {:?} (VTK version {}.{})",
        surface.part_name,
        surface.num_faces(),
        output_path,
        version.0,
        version.1
    );

    // Create point array from nodes
    let points: Vec<f64> = surface
        .nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity for quad faces
    let mut connectivity = Vec::new();
    for face in &surface.faces {
        connectivity.extend_from_slice(&face.node_ids.map(|id| id as u64));
    }

    // All cells are quads (VTK_QUAD = 9)
    let cell_types = vec![CellType::Quad; surface.faces.len()];

    // Create cells with offsets
    let cells = Cells {
        cell_verts: VertexNumbers::XML {
            connectivity,
            offsets: (0..surface.faces.len())
                .map(|i| ((i + 1) * 4) as u64)
                .collect(),
        },
        types: cell_types,
    };

    // Create unstructured grid piece
    let mut ugrid = UnstructuredGridPiece {
        points: IOBuffer::F64(points),
        cells,
        data: Attributes::new(),
    };

    // Add face normals as cell data (vectors with 3 components)
    let normal_data: Vec<f64> = surface
        .face_normals
        .iter()
        .flat_map(|n| vec![n.x, n.y, n.z])
        .collect();

    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "normals".into(),
        elem: ElementType::Vectors,
        data: IOBuffer::F64(normal_data),
    }));

    // Add face areas as cell data (scalars)
    let area_data: Vec<f64> = surface.face_areas.clone();

    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "area".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(area_data),
    }));

    // Create the Vtk model
    let vtk = Vtk {
        version: Version::new(version),
        title: format!("Surface mesh: {}", surface.part_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::UnstructuredGrid {
            pieces: vec![Piece::Inline(Box::new(ugrid))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write VTU file: {}", e)))?;

    log::info!("Successfully wrote VTU file to {:?}", output_path);

    Ok(())
}

/// Write multiple surface meshes to separate VTU files
/// Each surface is written to <output_dir>/<part_name>.vtu
pub fn write_surfaces_to_vtu(
    surfaces: &[SurfaceMesh],
    output_dir: &Path,
    vtk_version: Option<(u8, u8)>,
) -> Result<()> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;

    for surface in surfaces {
        let filename = format!("{}.vtu", sanitize_filename(&surface.part_name));
        let output_path = output_dir.join(filename);
        write_surface_to_vtu(surface, &output_path, vtk_version)?;
    }

    Ok(())
}

/// Write surface mesh with contact pair metadata to VTU
pub fn write_surface_with_contact_metadata(
    surface: &SurfaceMesh,
    results: &crate::contact::ContactResults,
    _metrics: &crate::contact::SurfaceMetrics,
    output_path: &Path,
    vtk_version: Option<(u8, u8)>,
) -> Result<()> {
    let version = vtk_version.unwrap_or(DEFAULT_VTK_VERSION);
    log::info!(
        "Writing surface '{}' with contact metadata to {:?} (VTK version {}.{})",
        surface.part_name,
        output_path,
        version.0,
        version.1
    );

    // Create point array from nodes
    let points: Vec<f64> = surface
        .nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity for quad faces
    let mut connectivity = Vec::new();
    for face in &surface.faces {
        connectivity.extend_from_slice(&face.node_ids.map(|id| id as u64));
    }

    // All cells are quads
    let cell_types = vec![CellType::Quad; surface.faces.len()];

    // Create cells
    let cells = Cells {
        cell_verts: VertexNumbers::XML {
            connectivity,
            offsets: (0..surface.faces.len())
                .map(|i| ((i + 1) * 4) as u64)
                .collect(),
        },
        types: cell_types,
    };

    // Create unstructured grid piece
    let mut ugrid = UnstructuredGridPiece {
        points: IOBuffer::F64(points),
        cells,
        data: Attributes::new(),
    };

    // Add face normals as cell data
    let normal_data: Vec<f64> = surface
        .face_normals
        .iter()
        .flat_map(|n| vec![n.x, n.y, n.z])
        .collect();

    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "normals".into(),
        elem: ElementType::Vectors,
        data: IOBuffer::F64(normal_data),
    }));

    // Add face areas
    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "area".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(surface.face_areas.clone()),
    }));

    // Create a map from face index to contact pair
    let mut face_to_pair = vec![-1i32; surface.faces.len()];
    let mut face_distance = vec![0.0f64; surface.faces.len()];
    let mut face_angle = vec![0.0f64; surface.faces.len()];

    for (pair_idx, pair) in results.pairs.iter().enumerate() {
        face_to_pair[pair.surface_a_face_id] = pair_idx as i32;
        face_distance[pair.surface_a_face_id] = pair.distance;
        face_angle[pair.surface_a_face_id] = pair.normal_angle;
    }

    // Add contact pair ID as cell data
    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "pair_id".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(face_to_pair),
    }));

    // Add distance as cell data
    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "distance".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(face_distance),
    }));

    // Add normal angle as cell data
    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "normal_angle".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(face_angle),
    }));

    // Note: Surface-level metrics are printed to console and can be accessed via the metrics parameter
    // VTK file format limitations prevent easy embedding of arbitrary metadata
    // Cell data (per-face data) is included above

    // Create the Vtk model
    let vtk = Vtk {
        version: Version::new(version),
        title: format!("Surface mesh with contact data: {}", surface.part_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::UnstructuredGrid {
            pieces: vec![Piece::Inline(Box::new(ugrid))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write VTU file: {}", e)))?;

    log::info!(
        "Successfully wrote VTU file with contact metadata to {:?}",
        output_path
    );

    Ok(())
}

/// Sanitize a string to be a valid filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Write a full mesh (with hex elements) to a VTK file
///
/// This is useful for visualizing synthetic meshes or full 3D meshes.
pub fn write_vtk(mesh: &Mesh, output_path: &Path, vtk_version: Option<(u8, u8)>) -> Result<()> {
    let version = vtk_version.unwrap_or(DEFAULT_VTK_VERSION);
    log::info!(
        "Writing mesh with {} elements to {:?} (VTK version {}.{})",
        mesh.num_elements(),
        output_path,
        version.0,
        version.1
    );

    // Create point array from nodes
    let points: Vec<f64> = mesh
        .nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity for hex elements
    let mut connectivity = Vec::new();
    for elem in &mesh.elements {
        connectivity.extend_from_slice(&elem.node_ids.map(|id| id as u64));
    }

    // All cells are hexes (VTK_HEXAHEDRON = 12)
    let cell_types = vec![CellType::Hexahedron; mesh.elements.len()];

    // Create cells with offsets (each hex has 8 nodes)
    let cells = Cells {
        cell_verts: VertexNumbers::XML {
            connectivity,
            offsets: (0..mesh.elements.len())
                .map(|i| ((i + 1) * 8) as u64)
                .collect(),
        },
        types: cell_types,
    };

    // Create unstructured grid piece
    let ugrid = UnstructuredGridPiece {
        points: IOBuffer::F64(points),
        cells,
        data: Attributes::new(),
    };

    // Create the Vtk model
    let vtk = Vtk {
        version: Version::new(version),
        title: "Hexahedral mesh".to_string(),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::UnstructuredGrid {
            pieces: vec![Piece::Inline(Box::new(ugrid))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write VTK file: {}", e)))?;

    log::info!("Successfully wrote VTK file to {:?}", output_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::types::{Point, QuadFace, Vec3};

    fn make_test_surface() -> SurfaceMesh {
        let nodes = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
            Point::new(0.0, 1.0, 0.0),
        ];

        let face = QuadFace::new([0, 1, 2, 3]);

        SurfaceMesh {
            part_name: "TestBlock".to_string(),
            faces: vec![face],
            face_normals: vec![Vec3::new(0.0, 0.0, 1.0)],
            face_centroids: vec![Point::new(0.5, 0.5, 0.0)],
            face_areas: vec![1.0],
            nodes,
        }
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Block 1"), "Block_1");
        assert_eq!(sanitize_filename("Part-A/B"), "Part-A_B");
        assert_eq!(sanitize_filename("Normal_Name"), "Normal_Name");
    }

    #[test]
    fn test_write_surface_to_vtu() {
        let surface = make_test_surface();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_surface.vtu");

        let result = write_surface_to_vtu(&surface, &output_path, None);
        assert!(result.is_ok());

        // Clean up
        let _ = std::fs::remove_file(&output_path);
    }
}
