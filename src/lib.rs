//! Contact Detector Library
//!
//! High-performance hexahedral mesh contact pair detection and surface extraction.

pub mod config;
pub mod contact;
pub mod error;
pub mod io;
pub mod mesh;

pub use error::{ContactDetectorError, Result};
