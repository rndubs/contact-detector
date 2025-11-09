//! I/O module for reading and writing mesh files

#[cfg(feature = "exodus")]
pub mod exodus;

pub mod json;
pub mod metadata;
pub mod vtu;
pub mod vtm;

#[cfg(feature = "exodus")]
pub use exodus::{add_contact_sidesets_to_mesh, surface_to_sideset, write_exodus, ExodusReader};

pub use json::{read_json_mesh, write_json_mesh};
pub use metadata::ContactMetadata;
pub use vtu::{
    write_contact_surfaces_with_skin, write_surface_to_vtu, write_surface_with_contact_metadata,
    write_surfaces_to_vtu, write_vtk,
};
pub use vtm::MultiBlockBuilder;
