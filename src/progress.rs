//! Progress reporting for archive operations.

/// Reports a packed member.
pub fn packed(name: &str, size: u32) {
    eprintln!("packed {name} ({size} bytes)");
}

/// Reports an extracted member.
pub fn extracted(name: &str, size: u32) {
    eprintln!("extracted {name} ({size} bytes)");
}
