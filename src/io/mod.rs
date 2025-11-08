//! I/O module for reading and writing mesh files

#[cfg(feature = "exodus")]
pub mod exodus;

pub mod json;
pub mod vtu;

#[cfg(feature = "exodus")]
pub use exodus::{write_exodus, ExodusReader};

pub use json::{read_json_mesh, write_json_mesh};
pub use vtu::{write_surface_to_vtu, write_surface_with_contact_metadata, write_surfaces_to_vtu, write_vtk};
