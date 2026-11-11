//! Library components for reading, writing, and operating on NORK archives.
#![warn(missing_docs)]

pub mod archive;
pub mod cli;
pub mod errors;
pub mod filesystem;
pub mod operations;
pub mod progress;

/// Compatibility alias for the pre-refactor operations module.
pub use operations as file_ops;
