//! Error types for the contact detector application

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContactDetectorError {
    #[error("Failed to read Exodus file: {0}")]
    ExodusReadError(String),

    #[error("Invalid mesh topology: {0}")]
    InvalidMeshTopology(String),

    #[error("Element block not found: {0}")]
    ElementBlockNotFound(String),

    #[error("Invalid element type: expected {expected}, found {found}")]
    InvalidElementType { expected: String, found: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("NetCDF error: {0}")]
    NetcdfError(String),

    #[error("VTK error: {0}")]
    VtkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Geometry error: {0}")]
    GeometryError(String),
}

pub type Result<T> = std::result::Result<T, ContactDetectorError>;
