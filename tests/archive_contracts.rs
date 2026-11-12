use sarges_unpacker::archive::{Archive, ArchiveWriter};

#[test]
fn parses_nork_header_and_verbatim_payload() {
    let mut bytes = Vec::new();
    ArchiveWriter::new()
        .add("hello.txt", b"hello")
        .write_to(&mut bytes)
        .unwrap();
    let archive = Archive::parse(&bytes).unwrap();
    assert_eq!(archive.entries(), &["hello.txt"]);
    assert_eq!(archive.payload(&bytes, "hello.txt").unwrap(), b"hello");
}

#[test]
fn rejects_unsupported_version() {
    let mut bytes = Vec::new();
    ArchiveWriter::new()
        .add("x", b"x")
        .write_to(&mut bytes)
        .unwrap();
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
    assert!(Archive::parse(&bytes).is_err());
}

#[test]
fn rejects_truncated_archive_size() {
    let mut bytes = Vec::new();
    ArchiveWriter::new()
        .add("x", b"x")
        .write_to(&mut bytes)
        .unwrap();
    bytes[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Archive::parse(&bytes).is_err());
}

#[test]
fn accepts_archive_with_trailing_bytes_like_reference_tool() {
    let mut bytes = Vec::new();
    ArchiveWriter::new()
        .add("x", b"x")
        .write_to(&mut bytes)
        .unwrap();
    bytes.extend_from_slice(b"trailing data");

    let archive = Archive::parse(&bytes).unwrap();
    assert_eq!(archive.entries(), vec!["x"]);
}

#[test]
fn rejects_truncated_or_wrong_magic() {
    assert!(Archive::parse(b"NORK").is_err());
    assert!(Archive::parse(b"NOPE........................").is_err());
}

#[test]
fn rejects_common_mutations_without_panicking() {
    let mut valid = Vec::new();
    ArchiveWriter::new()
        .add("member", b"payload")
        .write_to(&mut valid)
        .unwrap();
    for index in 0..valid.len() {
        let mut mutated = valid.clone();
        mutated[index] ^= 0xA5;
        let _ = Archive::parse(&mutated);
    }
}
