//! Shared constructors for user-facing I/O errors.

use std::io;

/// Creates an invalid-input error with the supplied message.
pub fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// Creates the standard archive-size-limit error.
pub fn archive_too_large() -> io::Error {
    invalid_input("archive is too large")
}

/// Creates the standard member-size-limit error.
pub fn member_too_large() -> io::Error {
    invalid_input("member is too large")
}
