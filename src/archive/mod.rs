//! NORK archive parsing and serialization.

use std::io;

/// Number of bytes in a NORK v1 archive header.
pub const HEADER_SIZE: usize = 56;
const MAGIC: &[u8; 4] = b"NORK";

#[derive(Debug, Clone, PartialEq, Eq)]
/// A member entry from an archive directory.
pub struct Entry {
    /// UTF-8 member path stored in the archive.
    pub name: String,
    /// Absolute byte offset of the member payload.
    pub offset: u32,
    /// Member payload length in bytes.
    pub size: u32,
}

#[derive(Debug, Clone)]
/// Parsed NORK archive metadata.
pub struct Archive {
    entries: Vec<Entry>,
}

#[derive(Default)]
/// Builder for creating NORK v1 archives in memory.
pub struct ArchiveWriter {
    files: Vec<(String, Vec<u8>)>,
}

fn u32le(bytes: &[u8], at: usize) -> io::Result<u32> {
    bytes
        .get(at..at + 4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "archive field is truncated"))
        .map(|x| u32::from_le_bytes(x.try_into().expect("four-byte slice is guaranteed by get")))
}

impl Archive {
    /// Parses and validates a complete NORK v1 archive.
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "archive header is truncated",
            ));
        }
        if bytes.get(..4) != Some(MAGIC) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid archive signature",
            ));
        }
        if u32le(bytes, 4)? != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported archive version",
            ));
        }
        if u32le(bytes, 12)? as usize != bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "archive size does not match input",
            ));
        }
        let count = u32le(bytes, 16)? as usize;
        let mut p = u32le(bytes, 20)? as usize;
        if p < HEADER_SIZE || p > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "directory offset is outside archive",
            ));
        }
        let mut entries = Vec::with_capacity(count.min(1_000_000));
        for _ in 0..count {
            p = p
                .checked_add(4)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "directory overflow"))?;
            let len = *bytes.get(p).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "directory is truncated")
            })? as usize;
            p += 1;
            let end_name = p.checked_add(len).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "member name overflow")
            })?;
            let name = String::from_utf8(
                bytes
                    .get(p..end_name)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "member name is truncated")
                    })?
                    .to_vec(),
            )
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "member name is not UTF-8"))?;
            p = end_name;
            let offset = u32le(bytes, p)?;
            let size = u32le(bytes, p + 4)?;
            p += 8;
            let end = (offset as usize)
                .checked_add(size as usize)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "member payload overflow")
                })?;
            if end > bytes.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "member payload is outside archive",
                ));
            }
            entries.push(Entry { name, offset, size });
        }
        Ok(Self { entries })
    }

    /// Returns member names in directory order.
    pub fn entries(&self) -> Vec<String> {
        self.entries.iter().map(|x| x.name.clone()).collect()
    }
    /// Returns directory records in directory order.
    pub fn records(&self) -> &[Entry] {
        &self.entries
    }
    /// Returns the payload for a named member.
    pub fn payload<'a>(&self, bytes: &'a [u8], name: &str) -> io::Result<&'a [u8]> {
        let e = self
            .entries
            .iter()
            .find(|x| x.name == name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "archive member not found"))?;
        Ok(&bytes[e.offset as usize..(e.offset + e.size) as usize])
    }
}

impl ArchiveWriter {
    /// Creates an empty archive builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a member and returns the builder for fluent construction.
    pub fn add(mut self, name: impl Into<String>, data: &[u8]) -> Self {
        self.files.push((name.into(), data.to_vec()));
        self
    }
    /// Serializes the archive into the supplied output buffer.
    pub fn write_to(self, out: &mut Vec<u8>) -> io::Result<()> {
        let file_count = self.files.len();
        out.resize(HEADER_SIZE, 0);
        let mut records = Vec::new();
        for (name, data) in self.files {
            let offset = u32::try_from(out.len())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "archive is too large"))?;
            let size = u32::try_from(data.len())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "member is too large"))?;
            out.extend_from_slice(&data);
            records.push((name, offset, size));
        }
        let directory = u32::try_from(out.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "archive is too large"))?;
        for (name, offset, size) in records {
            let name = name.as_bytes();
            if name.len() > 255 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "member name is too long",
                ));
            }
            out.extend_from_slice(&0u32.to_le_bytes());
            out.push(name.len() as u8);
            out.extend_from_slice(name);
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
        }
        let total = u32::try_from(out.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "archive is too large"))?;
        out[0..4].copy_from_slice(MAGIC);
        out[4..8].copy_from_slice(&1u32.to_le_bytes());
        out[12..16].copy_from_slice(&total.to_le_bytes());
        out[16..20].copy_from_slice(&(file_count as u32).to_le_bytes());
        out[20..24].copy_from_slice(&directory.to_le_bytes());
        Ok(())
    }
}
