//! Filesystem traversal and safe archive path handling.

use crate::errors;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Controls which filesystem entries are collected for an archive.
pub struct CollectionOptions<'a> {
    /// Include files below nested directories.
    pub recurse: bool,
    /// Optional case-insensitive file-name mask.
    pub mask: Option<&'a str>,
}

fn matches(name: &str, pattern: &str) -> bool {
    fn visit(name: &[u8], pattern: &[u8]) -> bool {
        if pattern.is_empty() {
            return name.is_empty();
        }
        if pattern[0] == b'*' {
            return visit(name, &pattern[1..]) || (!name.is_empty() && visit(&name[1..], pattern));
        }
        !name.is_empty()
            && (pattern[0] == b'?' || pattern[0].eq_ignore_ascii_case(&name[0]))
            && visit(&name[1..], &pattern[1..])
    }

    visit(name.as_bytes(), pattern.as_bytes())
}

/// Collects eligible files and returns deterministic archive member paths.
pub fn collect_files(
    root: &Path,
    base: &Path,
    options: &CollectionOptions<'_>,
) -> io::Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    collect(root, base, options, &mut files, &mut seen, 0)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn collect(
    root: &Path,
    base: &Path,
    options: &CollectionOptions<'_>,
    output: &mut Vec<(String, PathBuf)>,
    seen: &mut HashSet<String>,
    depth: usize,
) -> io::Result<()> {
    if depth > 64 {
        return Err(errors::invalid_input(
            "directory nesting exceeds supported limit",
        ));
    }
    for item in fs::read_dir(root)? {
        let item = item?;
        let name = item.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let path = item.path();
        if path.is_dir() {
            if options.recurse {
                collect(&path, base, options, output, seen, depth + 1)?;
            }
            continue;
        }
        if options
            .mask
            .is_some_and(|mask| !matches(&name.to_string_lossy(), mask))
        {
            continue;
        }
        let member = path
            .strip_prefix(base)
            .expect("collected path must be below its collection root")
            .to_string_lossy()
            .replace('\\', "/");
        if member.len() > 255 {
            return Err(errors::invalid_input(
                "member path exceeds 255-byte archive limit",
            ));
        }
        if !seen.insert(name.to_string_lossy().to_ascii_lowercase()) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("duplicate archive member: {member}"),
            ));
        }
        output.push((member, path));
    }
    Ok(())
}

/// Resolves a member below `root`, rejecting absolute and parent-traversal paths.
pub fn safe_member_path(root: &Path, name: &str) -> io::Result<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute()
        || path.has_root()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(errors::invalid_input("archive member escapes destination"));
    }
    Ok(root.join(path))
}
