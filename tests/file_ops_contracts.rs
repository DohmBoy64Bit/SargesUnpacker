use sarges_unpacker::file_ops::{
    create, extract, extract_with_options, CreateOptions, ExecutionOptions, ExistingFilePolicy,
};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("sarges-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn skips_dot_prefixed_files_and_rejects_duplicate_members() {
    let root = fixture("traversal");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".hidden"), b"hidden").unwrap();
    fs::write(root.join("same.bin"), b"one").unwrap();
    fs::write(root.join("sub/same.bin"), b"two").unwrap();
    let archive_path = root.join("out.x");
    let result = create(
        &root,
        &archive_path,
        CreateOptions {
            recurse: true,
            mask: None,
        },
    );
    assert!(result.is_err());
}

#[test]
fn rejects_parent_traversal_during_extraction() {
    let root = fixture("safe-extract");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("../escape.txt", b"x")
        .write_to(&mut bytes)
        .unwrap();
    let archive_path = root.join("bad.x");
    fs::write(&archive_path, bytes).unwrap();
    assert!(extract(&archive_path, &root.join("out")).is_err());
    assert!(!root.join("escape.txt").exists());
}

#[test]
fn rejects_excessive_directory_nesting() {
    let root = fixture("depth-limit");
    let mut current = root.clone();
    for index in 0..66 {
        current = current.join(format!("d{index}"));
        fs::create_dir(&current).unwrap();
    }
    fs::write(current.join("file.bin"), b"x").unwrap();
    assert!(create(
        &root,
        &root.join("out.x"),
        CreateOptions {
            recurse: true,
            mask: None
        }
    )
    .is_err());
}

#[test]
fn lists_member_names_sizes_and_offsets() {
    let root = fixture("list");
    let archive_path = root.join("sample.x");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("alpha.txt", b"abc")
        .add("nested/beta.bin", b"12345")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, bytes).unwrap();

    let listing = sarges_unpacker::file_ops::list(&archive_path).unwrap();
    assert!(listing.contains("alpha.txt"));
    assert!(listing.contains("nested/beta.bin"));
    assert!(listing.contains("3 bytes"));
    assert!(listing.contains("5 bytes"));
}

#[test]
fn verifies_a_valid_archive_without_extracting() {
    let root = fixture("verify");
    let archive_path = root.join("sample.x");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("file.bin", b"payload")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, bytes).unwrap();
    let report = sarges_unpacker::file_ops::verify(&archive_path).unwrap();
    assert!(report.contains("Archive verified"));
    assert!(report.contains("1 members"));
}

#[test]
fn extraction_policies_control_existing_files() {
    let root = fixture("overwrite-policy");
    let archive_path = root.join("sample.x");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("file.txt", b"new")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, bytes).unwrap();
    let output = root.join("out");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("file.txt"), b"old").unwrap();
    assert!(extract(&archive_path, &output).is_err());
    extract_with_options(
        &archive_path,
        &output,
        ExecutionOptions {
            dry_run: false,
            existing: ExistingFilePolicy::Skip,
            progress: false,
        },
    )
    .unwrap();
    assert_eq!(fs::read(output.join("file.txt")).unwrap(), b"old");
    extract_with_options(
        &archive_path,
        &output,
        ExecutionOptions {
            dry_run: false,
            existing: ExistingFilePolicy::Overwrite,
            progress: false,
        },
    )
    .unwrap();
    assert_eq!(fs::read(output.join("file.txt")).unwrap(), b"new");
}

#[test]
fn dry_run_does_not_create_archive_or_extract_files() {
    let root = fixture("dry-run");
    fs::write(root.join("source.txt"), b"source").unwrap();
    let archive_path = root.join("planned.x");
    sarges_unpacker::file_ops::create_with_options(
        &root,
        &archive_path,
        CreateOptions {
            recurse: false,
            mask: None,
        },
        ExecutionOptions {
            dry_run: true,
            existing: ExistingFilePolicy::Refuse,
            progress: false,
        },
    )
    .unwrap();
    assert!(!archive_path.exists());

    let output = root.join("planned-out");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("file.txt", b"x")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, bytes).unwrap();
    extract_with_options(
        &archive_path,
        &output,
        ExecutionOptions {
            dry_run: true,
            existing: ExistingFilePolicy::Refuse,
            progress: false,
        },
    )
    .unwrap();
    assert!(!output.exists());
}

#[test]
fn creation_orders_members_deterministically() {
    let root = fixture("ordering");
    fs::write(root.join("z.txt"), b"z").unwrap();
    fs::write(root.join("a.txt"), b"a").unwrap();
    let archive_path = root.join("ordered.x");
    create(
        &root,
        &archive_path,
        CreateOptions {
            recurse: false,
            mask: None,
        },
    )
    .unwrap();
    let bytes = fs::read(archive_path).unwrap();
    let archive = sarges_unpacker::archive::Archive::parse(&bytes).unwrap();
    assert_eq!(archive.entries(), vec!["a.txt", "z.txt"]);
}
