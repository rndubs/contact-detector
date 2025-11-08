//! I/O module for reading and writing mesh files

#[cfg(feature = "exodus")]
pub mod exodus;

pub mod json;

#[cfg(feature = "exodus")]
pub use exodus::ExodusReader;

pub use json::{read_json_mesh, write_json_mesh};
