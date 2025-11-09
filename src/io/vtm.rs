//! VTM (VTK Multi-block) file writer
//!
//! This module provides functionality for writing hierarchical multi-block VTK datasets (.vtm)
//! with support for element blocks, sidesets, nodesets, and contact pairs.

use crate::error::{ContactDetectorError, Result};
use crate::mesh::types::{Mesh, SurfaceMesh};
use std::fs;
use std::path::{Path, PathBuf};
use vtkio::model::*;

/// Multi-block dataset builder for organizing VTK output
pub struct MultiBlockBuilder {
    /// Root output directory
    output_dir: PathBuf,

    /// Base name for the multi-block file
    base_name: String,

    /// VTK version to use
    vtk_version: (u8, u8),

    /// Blocks to include in the multi-block dataset
    blocks: Vec<Block>,
}

/// Represents a block in the multi-block hierarchy
#[derive(Debug, Clone)]
struct Block {
    /// Block name
    name: String,

    /// Relative path to the VTK file
    file_path: PathBuf,

    /// Child blocks (for nested hierarchies)
    children: Vec<Block>,
}

impl MultiBlockBuilder {
    /// Create a new multi-block builder
    pub fn new<P: AsRef<Path>>(output_dir: P, base_name: String, vtk_version: (u8, u8)) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            base_name,
            vtk_version,
            blocks: Vec::new(),
        }
    }

    /// Add a volume mesh block (element blocks)
    pub fn add_volume_mesh(&mut self, mesh: &Mesh) -> Result<()> {
        log::info!("Adding volume mesh blocks to multi-block dataset");

        // Create volume directory
        let volume_dir = self.output_dir.join("volume");
        fs::create_dir_all(&volume_dir)?;

        let mut volume_blocks = Vec::new();

        // Export each element block as a separate VTU file
        for (block_name, element_indices) in &mesh.element_blocks {
            let filename = format!("{}.vtu", sanitize_filename(block_name));
            let file_path = volume_dir.join(&filename);
            let rel_path = PathBuf::from("volume").join(&filename);

            // Write the element block
            write_element_block(mesh, block_name, element_indices, &file_path, self.vtk_version)?;

            volume_blocks.push(Block {
                name: block_name.clone(),
                file_path: rel_path,
                children: Vec::new(),
            });
        }

        // Add volume mesh parent block
        if !volume_blocks.is_empty() {
            self.blocks.push(Block {
                name: "VolumeMesh".to_string(),
                file_path: PathBuf::new(), // Parent block has no file
                children: volume_blocks,
            });
        }

        Ok(())
    }

    /// Add sideset blocks (boundary surfaces)
    pub fn add_sidesets(&mut self, mesh: &Mesh) -> Result<()> {
        if mesh.side_sets.is_empty() {
            log::debug!("No sidesets to export");
            return Ok(());
        }

        log::info!("Adding {} sidesets to multi-block dataset", mesh.side_sets.len());

        // Create sidesets directory
        let sidesets_dir = self.output_dir.join("sidesets");
        fs::create_dir_all(&sidesets_dir)?;

        let mut sideset_blocks = Vec::new();

        for (sideset_name, sideset_data) in &mesh.side_sets {
            let filename = format!("{}.vtp", sanitize_filename(sideset_name));
            let file_path = sidesets_dir.join(&filename);
            let rel_path = PathBuf::from("sidesets").join(&filename);

            // Write the sideset as polydata
            write_sideset_polydata(mesh, sideset_name, sideset_data, &file_path, self.vtk_version)?;

            sideset_blocks.push(Block {
                name: format!("Sideset_{}", sideset_name),
                file_path: rel_path,
                children: Vec::new(),
            });
        }

        // Add sidesets parent block
        if !sideset_blocks.is_empty() {
            self.blocks.push(Block {
                name: "Sidesets".to_string(),
                file_path: PathBuf::new(),
                children: sideset_blocks,
            });
        }

        Ok(())
    }

    /// Add nodeset blocks (point sets)
    pub fn add_nodesets(&mut self, mesh: &Mesh) -> Result<()> {
        if mesh.node_sets.is_empty() {
            log::debug!("No nodesets to export");
            return Ok(());
        }

        log::info!("Adding {} nodesets to multi-block dataset", mesh.node_sets.len());

        // Create nodesets directory
        let nodesets_dir = self.output_dir.join("nodesets");
        fs::create_dir_all(&nodesets_dir)?;

        let mut nodeset_blocks = Vec::new();

        for (nodeset_name, node_indices) in &mesh.node_sets {
            let filename = format!("{}.vtp", sanitize_filename(nodeset_name));
            let file_path = nodesets_dir.join(&filename);
            let rel_path = PathBuf::from("nodesets").join(&filename);

            // Write the nodeset as vertex polydata
            write_nodeset_polydata(mesh, nodeset_name, node_indices, &file_path, self.vtk_version)?;

            nodeset_blocks.push(Block {
                name: format!("Nodeset_{}", nodeset_name),
                file_path: rel_path,
                children: Vec::new(),
            });
        }

        // Add nodesets parent block
        if !nodeset_blocks.is_empty() {
            self.blocks.push(Block {
                name: "Nodesets".to_string(),
                file_path: PathBuf::new(),
                children: nodeset_blocks,
            });
        }

        Ok(())
    }

    /// Add contact pair blocks with metadata
    pub fn add_contact_pairs(
        &mut self,
        contact_pairs: &[(String, String, SurfaceMesh, SurfaceMesh, crate::contact::ContactResults)],
        pair_id_offset: usize,
    ) -> Result<()> {
        if contact_pairs.is_empty() {
            log::debug!("No contact pairs to export");
            return Ok(());
        }

        log::info!("Adding {} contact pairs to multi-block dataset", contact_pairs.len());

        // Create contact pairs directory
        let contact_dir = self.output_dir.join("contact_pairs");
        fs::create_dir_all(&contact_dir)?;

        let mut contact_blocks = Vec::new();

        for (idx, (surf_a_name, surf_b_name, surf_a, surf_b, results)) in contact_pairs.iter().enumerate() {
            let pair_id = pair_id_offset + idx;

            // Create a sub-block for this contact pair
            let mut pair_blocks = Vec::new();

            // Master surface (surface A)
            let master_filename = format!("ContactPair_{}_Master.vtp", pair_id);
            let master_file_path = contact_dir.join(&master_filename);
            let master_rel_path = PathBuf::from("contact_pairs").join(&master_filename);

            write_contact_surface_polydata(
                surf_a,
                results,
                pair_id,
                0, // ContactRole: 0 = master
                &master_file_path,
                self.vtk_version,
            )?;

            pair_blocks.push(Block {
                name: format!("Master_{}", surf_a_name),
                file_path: master_rel_path,
                children: Vec::new(),
            });

            // Slave surface (surface B)
            let slave_filename = format!("ContactPair_{}_Slave.vtp", pair_id);
            let slave_file_path = contact_dir.join(&slave_filename);
            let slave_rel_path = PathBuf::from("contact_pairs").join(&slave_filename);

            write_contact_surface_polydata(
                surf_b,
                results,
                pair_id,
                1, // ContactRole: 1 = slave
                &slave_file_path,
                self.vtk_version,
            )?;

            pair_blocks.push(Block {
                name: format!("Slave_{}", surf_b_name),
                file_path: slave_rel_path,
                children: Vec::new(),
            });

            // Add this contact pair as a parent block
            contact_blocks.push(Block {
                name: format!("ContactPair_{}", pair_id),
                file_path: PathBuf::new(),
                children: pair_blocks,
            });
        }

        // Add contact pairs parent block
        if !contact_blocks.is_empty() {
            self.blocks.push(Block {
                name: "ContactPairs".to_string(),
                file_path: PathBuf::new(),
                children: contact_blocks,
            });
        }

        Ok(())
    }

    /// Write the multi-block meta file (.vtm)
    pub fn write(&self) -> Result<()> {
        let vtm_path = self.output_dir.join(format!("{}.vtm", self.base_name));
        log::info!("Writing multi-block meta file to {:?}", vtm_path);

        // Build XML content
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\"?>\n");
        xml.push_str(&format!(
            "<VTKFile type=\"vtkMultiBlockDataSet\" version=\"{}.{}\" byte_order=\"LittleEndian\">\n",
            self.vtk_version.0, self.vtk_version.1
        ));
        xml.push_str("  <vtkMultiBlockDataSet>\n");

        // Write all blocks
        for (idx, block) in self.blocks.iter().enumerate() {
            write_block_xml(&mut xml, block, idx, 2);
        }

        xml.push_str("  </vtkMultiBlockDataSet>\n");
        xml.push_str("</VTKFile>\n");

        // Write to file
        fs::write(&vtm_path, xml)?;

        log::info!("Successfully wrote multi-block meta file");
        Ok(())
    }
}

/// Write a block to XML with proper indentation
fn write_block_xml(xml: &mut String, block: &Block, index: usize, indent_level: usize) {
    let indent = "  ".repeat(indent_level);

    if block.children.is_empty() {
        // Leaf block with a file
        xml.push_str(&format!(
            "{}<Block index=\"{}\" name=\"{}\">\n",
            indent, index, block.name
        ));
        xml.push_str(&format!(
            "{}  <DataSet index=\"0\" file=\"{}\"/>\n",
            indent,
            block.file_path.display()
        ));
        xml.push_str(&format!("{}</Block>\n", indent));
    } else {
        // Parent block with children
        xml.push_str(&format!(
            "{}<Block index=\"{}\" name=\"{}\">\n",
            indent, index, block.name
        ));

        for (child_idx, child) in block.children.iter().enumerate() {
            write_block_xml(xml, child, child_idx, indent_level + 1);
        }

        xml.push_str(&format!("{}</Block>\n", indent));
    }
}

/// Write an element block to VTU file
fn write_element_block(
    mesh: &Mesh,
    block_name: &str,
    element_indices: &[usize],
    output_path: &Path,
    vtk_version: (u8, u8),
) -> Result<()> {
    log::debug!("Writing element block '{}' with {} elements", block_name, element_indices.len());

    // Collect unique nodes used by this block
    let mut node_map = std::collections::HashMap::new();
    let mut local_nodes = Vec::new();

    for &elem_idx in element_indices {
        let elem = &mesh.elements[elem_idx];
        for &node_id in &elem.node_ids {
            if !node_map.contains_key(&node_id) {
                node_map.insert(node_id, local_nodes.len());
                local_nodes.push(mesh.nodes[node_id]);
            }
        }
    }

    // Create point array
    let points: Vec<f64> = local_nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity with remapped node IDs
    let mut connectivity = Vec::new();
    for &elem_idx in element_indices {
        let elem = &mesh.elements[elem_idx];
        for &node_id in &elem.node_ids {
            let local_id = node_map[&node_id];
            connectivity.push(local_id as u64);
        }
    }

    // All cells are hexahedra
    let cell_types = vec![CellType::Hexahedron; element_indices.len()];

    // Create cells
    let cells = Cells {
        cell_verts: VertexNumbers::XML {
            connectivity,
            offsets: (0..element_indices.len())
                .map(|i| ((i + 1) * 8) as u64)
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

    // Add ElementBlockId as cell data
    let block_id = element_indices.len(); // Simple ID based on size (could be improved)
    ugrid.data.cell.push(Attribute::DataArray(DataArray {
        name: "ElementBlockId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![block_id as i32; element_indices.len()]),
    }));

    // Add MaterialId if available
    if !mesh.material_ids.is_empty() {
        let material_ids: Vec<i32> = element_indices
            .iter()
            .map(|&idx| mesh.material_ids.get(idx).copied().unwrap_or(0))
            .collect();

        ugrid.data.cell.push(Attribute::DataArray(DataArray {
            name: "MaterialId".into(),
            elem: ElementType::Scalars {
                num_comp: 1,
                lookup_table: None,
            },
            data: IOBuffer::I32(material_ids),
        }));
    }

    // Create VTK model
    let vtk = Vtk {
        version: Version::new(vtk_version),
        title: format!("Element block: {}", block_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::UnstructuredGrid {
            pieces: vec![Piece::Inline(Box::new(ugrid))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write element block VTU: {}", e)))?;

    Ok(())
}

/// Write a sideset as polydata (.vtp)
fn write_sideset_polydata(
    mesh: &Mesh,
    sideset_name: &str,
    sideset_data: &[(usize, u8)],
    output_path: &Path,
    vtk_version: (u8, u8),
) -> Result<()> {
    log::debug!("Writing sideset '{}' with {} faces", sideset_name, sideset_data.len());

    // Collect unique nodes and build faces
    let mut node_map = std::collections::HashMap::new();
    let mut local_nodes = Vec::new();
    let mut faces = Vec::new();
    let mut source_elem_ids = Vec::new();
    let mut source_elem_sides = Vec::new();

    for &(elem_idx, face_id) in sideset_data {
        let elem = &mesh.elements[elem_idx];
        let elem_faces = elem.faces();
        let face = elem_faces[face_id as usize];

        // Remap node IDs to local indices
        let mut local_face = [0usize; 4];
        for (i, &node_id) in face.node_ids.iter().enumerate() {
            if !node_map.contains_key(&node_id) {
                node_map.insert(node_id, local_nodes.len());
                local_nodes.push(mesh.nodes[node_id]);
            }
            local_face[i] = node_map[&node_id];
        }

        faces.push(local_face);
        source_elem_ids.push(elem_idx as i32);
        source_elem_sides.push(face_id as i32);
    }

    // Create point array
    let points: Vec<f64> = local_nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity
    let mut connectivity = Vec::new();
    for face in &faces {
        connectivity.extend_from_slice(&face.map(|id| id as u64));
    }

    // Create cells as VertexNumbers for polydata
    let polys = VertexNumbers::XML {
        connectivity,
        offsets: (0..faces.len())
            .map(|i| ((i + 1) * 4) as u64)
            .collect(),
    };

    // Create polydata piece
    let mut polydata = PolyDataPiece {
        points: IOBuffer::F64(points),
        polys: Some(polys),
        verts: None,
        lines: None,
        strips: None,
        data: Attributes::new(),
    };

    // Add SideSetId
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "SideSetId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![0; faces.len()]), // All same sideset
    }));

    // Add SourceElementId
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "SourceElementId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(source_elem_ids),
    }));

    // Add SourceElementSide
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "SourceElementSide".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(source_elem_sides),
    }));

    // Create VTK model
    let vtk = Vtk {
        version: Version::new(vtk_version),
        title: format!("Sideset: {}", sideset_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::PolyData {
            pieces: vec![Piece::Inline(Box::new(polydata))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write sideset polydata: {}", e)))?;

    Ok(())
}

/// Write a nodeset as vertex polydata (.vtp)
fn write_nodeset_polydata(
    mesh: &Mesh,
    nodeset_name: &str,
    node_indices: &[usize],
    output_path: &Path,
    vtk_version: (u8, u8),
) -> Result<()> {
    log::debug!("Writing nodeset '{}' with {} nodes", nodeset_name, node_indices.len());

    // Create point array
    let points: Vec<f64> = node_indices
        .iter()
        .flat_map(|&idx| {
            let p = &mesh.nodes[idx];
            vec![p.x, p.y, p.z]
        })
        .collect();

    // Create vertex cells (one vertex per node)
    let connectivity: Vec<u64> = (0..node_indices.len() as u64).collect();
    let offsets: Vec<u64> = (1..=node_indices.len() as u64).collect();

    let verts = VertexNumbers::XML {
        connectivity,
        offsets,
    };

    // Create polydata piece
    let mut polydata = PolyDataPiece {
        points: IOBuffer::F64(points),
        verts: Some(verts),
        polys: None,
        lines: None,
        strips: None,
        data: Attributes::new(),
    };

    // Add NodeSetId
    polydata.data.point.push(Attribute::DataArray(DataArray {
        name: "NodeSetId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![0; node_indices.len()]), // All same nodeset
    }));

    // Create VTK model
    let vtk = Vtk {
        version: Version::new(vtk_version),
        title: format!("Nodeset: {}", nodeset_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::PolyData {
            pieces: vec![Piece::Inline(Box::new(polydata))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write nodeset polydata: {}", e)))?;

    Ok(())
}

/// Write a contact surface as polydata with metadata
fn write_contact_surface_polydata(
    surface: &SurfaceMesh,
    results: &crate::contact::ContactResults,
    contact_pair_id: usize,
    contact_role: i32,
    output_path: &Path,
    vtk_version: (u8, u8),
) -> Result<()> {
    log::debug!(
        "Writing contact surface '{}' as polydata (pair_id={}, role={})",
        surface.part_name,
        contact_pair_id,
        contact_role
    );

    // Create point array
    let points: Vec<f64> = surface
        .nodes
        .iter()
        .flat_map(|p| vec![p.x, p.y, p.z])
        .collect();

    // Create cell connectivity
    let mut connectivity = Vec::new();
    for face in &surface.faces {
        connectivity.extend_from_slice(&face.node_ids.map(|id| id as u64));
    }

    // Create cells as VertexNumbers for polydata
    let polys = VertexNumbers::XML {
        connectivity,
        offsets: (0..surface.faces.len())
            .map(|i| ((i + 1) * 4) as u64)
            .collect(),
    };

    // Create polydata piece
    let mut polydata = PolyDataPiece {
        points: IOBuffer::F64(points),
        polys: Some(polys),
        verts: None,
        lines: None,
        strips: None,
        data: Attributes::new(),
    };

    // Add ContactSurfaceId
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "ContactSurfaceId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![contact_pair_id as i32; surface.faces.len()]),
    }));

    // Add ContactPairId
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "ContactPairId".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![contact_pair_id as i32; surface.faces.len()]),
    }));

    // Add ContactRole (0 = master, 1 = slave)
    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "ContactRole".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(vec![contact_role; surface.faces.len()]),
    }));

    // Add SurfaceNormal (3-component Float32 array)
    let normal_data: Vec<f32> = surface
        .face_normals
        .iter()
        .flat_map(|n| vec![n.x as f32, n.y as f32, n.z as f32])
        .collect();

    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "SurfaceNormal".into(),
        elem: ElementType::Vectors,
        data: IOBuffer::F32(normal_data),
    }));

    // Add distance and angle data
    let mut face_distance = vec![0.0f64; surface.faces.len()];
    let mut face_angle = vec![0.0f64; surface.faces.len()];
    let mut is_paired = vec![0i32; surface.faces.len()];

    for pair in &results.pairs {
        if pair.surface_a_face_id < surface.faces.len() {
            face_distance[pair.surface_a_face_id] = pair.distance;
            face_angle[pair.surface_a_face_id] = pair.normal_angle;
            is_paired[pair.surface_a_face_id] = 1;
        }
    }

    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "Distance".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(face_distance),
    }));

    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "NormalAngle".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::F64(face_angle),
    }));

    polydata.data.cell.push(Attribute::DataArray(DataArray {
        name: "IsPaired".into(),
        elem: ElementType::Scalars {
            num_comp: 1,
            lookup_table: None,
        },
        data: IOBuffer::I32(is_paired),
    }));

    // Create VTK model
    let vtk = Vtk {
        version: Version::new(vtk_version),
        title: format!("Contact surface: {}", surface.part_name),
        byte_order: ByteOrder::LittleEndian,
        data: DataSet::PolyData {
            pieces: vec![Piece::Inline(Box::new(polydata))],
            meta: None,
        },
        file_path: None,
    };

    // Write to file
    vtk.export(output_path)
        .map_err(|e| ContactDetectorError::VtkError(format!("Failed to write contact surface polydata: {}", e)))?;

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
