//! High-level archive create, extract, list, verify, and stats operations.

use crate::archive::{Archive, HEADER_SIZE};
use crate::filesystem::{self, CollectionOptions};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Filesystem selection options used by archive creation.
pub type CreateOptions<'a> = CollectionOptions<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Controls handling of existing extraction targets.
pub enum ExistingFilePolicy {
    /// Fail if the destination already exists.
    Refuse,
    /// Replace an existing destination.
    Overwrite,
    /// Leave an existing destination unchanged.
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Runtime behavior shared by create and extract operations.
pub struct ExecutionOptions {
    /// Avoid creating or modifying files.
    pub dry_run: bool,
    /// Existing-file behavior during extraction.
    pub existing: ExistingFilePolicy,
    /// Emit per-member progress messages.
    pub progress: bool,
}

/// Result summary for a batch archive extraction.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ExtractAllReport {
    /// Number of archives discovered.
    pub discovered: usize,
    /// Number of archives extracted successfully.
    pub extracted: usize,
    /// Archive paths and errors that could not be extracted.
    pub failures: Vec<String>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            existing: ExistingFilePolicy::Refuse,
            progress: false,
        }
    }
}

/// Creates an archive using default execution options.
pub fn create(input: &Path, output: &Path, options: CreateOptions<'_>) -> io::Result<()> {
    create_with_options(input, output, options, ExecutionOptions::default())
}

/// Creates an archive with explicit execution behavior.
pub fn create_with_options(
    input: &Path,
    output: &Path,
    options: CreateOptions<'_>,
    execution: ExecutionOptions,
) -> io::Result<()> {
    let files = filesystem::collect_files(input, input, &options)?;
    if execution.dry_run {
        return Ok(());
    }
    let temporary = output.with_extension("tmp");
    let mut stream = File::create(&temporary)?;
    stream.write_all(&[0u8; HEADER_SIZE])?;
    let mut records = Vec::with_capacity(files.len());
    for (name, path) in files {
        let offset = u32::try_from(stream.stream_position()?)
            .map_err(|_| crate::errors::archive_too_large())?;
        let mut input = File::open(path)?;
        let size = u32::try_from(io::copy(&mut input, &mut stream)?)
            .map_err(|_| crate::errors::member_too_large())?;
        if execution.progress {
            crate::progress::packed(&name, size);
        }
        records.push((name, offset, size));
    }
    let directory =
        u32::try_from(stream.stream_position()?).map_err(|_| crate::errors::archive_too_large())?;
    for (name, offset, size) in &records {
        let name = name.as_bytes();
        stream.write_all(&0u32.to_le_bytes())?;
        stream.write_all(&[name.len() as u8])?;
        stream.write_all(name)?;
        stream.write_all(&offset.to_le_bytes())?;
        stream.write_all(&size.to_le_bytes())?;
    }
    let total =
        u32::try_from(stream.stream_position()?).map_err(|_| crate::errors::archive_too_large())?;
    stream.seek(SeekFrom::Start(0))?;
    let mut header = [0u8; HEADER_SIZE];
    header[0..4].copy_from_slice(b"NORK");
    header[4..8].copy_from_slice(&1u32.to_le_bytes());
    header[12..16].copy_from_slice(&total.to_le_bytes());
    header[16..20].copy_from_slice(&(records.len() as u32).to_le_bytes());
    header[20..24].copy_from_slice(&directory.to_le_bytes());
    stream.write_all(&header)?;
    stream.sync_all()?;
    drop(stream);
    fs::rename(temporary, output)
}

/// Extracts an archive using default execution options.
pub fn extract(input: &Path, output: &Path) -> io::Result<()> {
    extract_with_options(input, output, ExecutionOptions::default())
}

/// Extracts an archive with explicit execution behavior.
pub fn extract_with_options(
    input: &Path,
    output: &Path,
    execution: ExecutionOptions,
) -> io::Result<()> {
    let mut bytes = Vec::new();
    File::open(input)?.read_to_end(&mut bytes)?;
    let archive = Archive::parse(&bytes)?;
    if !execution.dry_run {
        fs::create_dir_all(output)?;
    }
    let mut source = File::open(input)?;
    for entry in archive.records() {
        let path = filesystem::safe_member_path(output, &entry.name)?;
        if execution.dry_run {
            continue;
        }
        if path.exists() {
            match execution.existing {
                ExistingFilePolicy::Refuse => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("destination exists: {}", path.display()),
                    ));
                }
                ExistingFilePolicy::Skip => continue,
                ExistingFilePolicy::Overwrite => {}
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        source.seek(SeekFrom::Start(u64::from(entry.offset)))?;
        let mut limited = (&mut source).take(u64::from(entry.size));
        let mut destination = File::create(path)?;
        io::copy(&mut limited, &mut destination)?;
        if execution.progress {
            crate::progress::extracted(&entry.name, entry.size);
        }
    }
    Ok(())
}

/// Extracts all `.x` archives found in a directory.
pub fn extract_all(
    input: &Path,
    output: &Path,
    recurse: bool,
    execution: ExecutionOptions,
) -> io::Result<ExtractAllReport> {
    let archives = discover_archives(input, recurse)?;
    let mut report = ExtractAllReport {
        discovered: archives.len(),
        ..ExtractAllReport::default()
    };
    let mut output_names = HashSet::new();

    for archive in archives {
        let name = archive
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                crate::errors::invalid_input(format!(
                    "archive has no usable name: {}",
                    archive.display()
                ))
            })?;
        let key = name.to_ascii_lowercase();
        if !output_names.insert(key) {
            report.failures.push(format!(
                "{}: output directory name collision",
                archive.display()
            ));
            continue;
        }
        let destination = output.join(name);
        match extract_with_options(&archive, &destination, execution) {
            Ok(()) => report.extracted += 1,
            Err(error) => report
                .failures
                .push(format!("{}: {error}", archive.display())),
        }
    }
    Ok(report)
}

fn discover_archives(root: &Path, recurse: bool) -> io::Result<Vec<PathBuf>> {
    let mut archives = Vec::new();
    discover_archives_in(root, recurse, &mut archives)?;
    archives.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    Ok(archives)
}

fn discover_archives_in(root: &Path, recurse: bool, output: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if recurse {
                discover_archives_in(&path, recurse, output)?;
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("x"))
        {
            output.push(path);
        }
    }
    Ok(())
}

/// Formats archive members as human-readable text.
pub fn list(input: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(input)?.read_to_end(&mut bytes)?;
    let archive = Archive::parse(&bytes)?;
    let mut output = format!("Members: {}\n", archive.records().len());
    for (index, entry) in archive.records().iter().enumerate() {
        output.push_str(&format!(
            "{:>4} {:>10} bytes offset {:>10} {}\n",
            index + 1,
            entry.size,
            entry.offset,
            entry.name
        ));
    }
    Ok(output)
}

/// Formats archive members as JSON.
pub fn list_json(input: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(input)?.read_to_end(&mut bytes)?;
    let archive = Archive::parse(&bytes)?;
    let members = archive
        .records()
        .iter()
        .map(|entry| {
            format!(
                "{{\"name\":\"{}\",\"offset\":{},\"size\":{}}}",
                json_escape(&entry.name),
                entry.offset,
                entry.size
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("{{\"members\":[{}]}}", members))
}

/// Validates an archive and formats a human-readable result.
pub fn verify(input: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(input)?.read_to_end(&mut bytes)?;
    let archive = Archive::parse(&bytes)?;
    Ok(format!(
        "Archive verified: {} members, {} bytes\n",
        archive.records().len(),
        bytes.len()
    ))
}

/// Validates an archive and formats a JSON result.
pub fn verify_json(input: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(input)?.read_to_end(&mut bytes)?;
    let archive = Archive::parse(&bytes)?;
    Ok(format!(
        "{{\"status\":\"verified\",\"members\":{},\"bytes\":{}}}",
        archive.records().len(),
        bytes.len()
    ))
}

/// Reports archive statistics in human-readable form.
pub fn stats(input: &Path) -> io::Result<String> {
    verify(input)
}

/// Reports archive statistics in JSON form.
pub fn stats_json(input: &Path) -> io::Result<String> {
    verify_json(input)
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
