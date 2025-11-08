//! Error types for the contact detector application
//!
//! This module defines all error types that can occur during mesh reading,
//! surface extraction, and contact pair detection operations.

use thiserror::Error;

/// Error types for contact detection operations
///
/// This enum represents all possible errors that can occur during mesh reading,
/// surface extraction, and contact pair detection operations.
#[derive(Error, Debug)]
pub enum ContactDetectorError {
    /// Failed to read an Exodus II file
    ///
    /// This typically indicates a corrupted file, unsupported file version,
    /// or missing required data arrays.
    #[error("Failed to read Exodus file: {0}")]
    ExodusReadError(String),

    /// Mesh topology is invalid or corrupted
    ///
    /// This error occurs when the mesh data violates expected constraints,
    /// such as invalid node IDs, out-of-bounds connectivity, or degenerate elements.
    #[error("Invalid mesh topology: {0}")]
    InvalidMeshTopology(String),

    /// Requested element block (part) not found in mesh
    ///
    /// This occurs when a contact pair configuration references a part name
    /// that doesn't exist in the mesh file.
    #[error("Element block not found: {0}")]
    ElementBlockNotFound(String),

    /// Element type doesn't match expected type
    ///
    /// This tool only supports hexahedral (HEX8) elements. This error occurs
    /// when the mesh contains different element types.
    #[error("Invalid element type: expected {expected}, found {found}")]
    InvalidElementType { expected: String, found: String },

    /// File I/O error
    ///
    /// Wraps standard I/O errors from file operations.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// NetCDF library error
    ///
    /// Errors from the underlying NetCDF library when reading Exodus II files.
    #[error("NetCDF error: {0}")]
    NetcdfError(String),

    /// VTK file writing error
    ///
    /// Errors when writing VTU (VTK Unstructured Grid) output files.
    #[error("VTK error: {0}")]
    VtkError(String),

    /// Configuration error
    ///
    /// Invalid configuration file format, missing required fields,
    /// or invalid parameter values.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Geometric computation error
    ///
    /// Errors during geometric computations such as degenerate faces
    /// (zero-area or zero-length normals).
    #[error("Geometry error: {0}")]
    GeometryError(String),
}

/// Convenience type alias for Results with [`ContactDetectorError`]
///
/// This type alias is used throughout the codebase for cleaner error handling.
///
/// # Example
/// ```
/// use contact_detector::Result;
///
/// fn my_function() -> Result<()> {
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, ContactDetectorError>;
