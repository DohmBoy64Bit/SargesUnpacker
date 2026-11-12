use sarges_unpacker::file_ops::{
    add_files_to_archive, create, extract, extract_all, extract_with_options, AddFilesOptions,
    CreateOptions, ExecutionOptions, ExistingFilePolicy,
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

#[test]
fn extracts_all_archives_into_named_directories() {
    let root = fixture("extract-all");
    let archives = root.join("archives");
    let nested = archives.join("nested");
    fs::create_dir_all(&nested).unwrap();

    for (directory, name, contents) in [
        (&archives, "first.x", b"first".as_slice()),
        (&nested, "second.x", b"second".as_slice()),
    ] {
        let mut bytes = Vec::new();
        sarges_unpacker::archive::ArchiveWriter::new()
            .add("member.txt", contents)
            .write_to(&mut bytes)
            .unwrap();
        fs::write(directory.join(name), bytes).unwrap();
    }

    let output = root.join("output");
    let report = extract_all(&archives, &output, false, ExecutionOptions::default()).unwrap();

    assert_eq!(report.discovered, 1);
    assert_eq!(report.extracted, 1);
    assert!(report.failures.is_empty());
    assert_eq!(fs::read(output.join("first/member.txt")).unwrap(), b"first");
    assert!(!output.join("second").exists());
}

#[test]
fn recursively_extracts_all_archives_and_supports_dry_run() {
    let root = fixture("extract-all-recursive");
    let archives = root.join("archives");
    let nested = archives.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("member.txt", b"nested")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(nested.join("nested.x"), bytes).unwrap();

    let dry_output = root.join("dry-output");
    let dry_report = extract_all(
        &archives,
        &dry_output,
        true,
        ExecutionOptions {
            dry_run: true,
            ..ExecutionOptions::default()
        },
    )
    .unwrap();
    assert_eq!(dry_report.discovered, 1);
    assert_eq!(dry_report.extracted, 1);
    assert!(!dry_output.exists());

    let output = root.join("output");
    extract_all(&archives, &output, true, ExecutionOptions::default()).unwrap();
    assert_eq!(
        fs::read(output.join("nested/nested/member.txt")).unwrap(),
        b"nested"
    );
}

#[test]
fn extract_all_reports_failures_without_stopping_other_archives() {
    let root = fixture("extract-all-failures");
    let archives = root.join("archives");
    fs::create_dir_all(&archives).unwrap();
    fs::write(archives.join("bad.x"), b"not an archive").unwrap();
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("member.txt", b"good")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(archives.join("good.x"), bytes).unwrap();

    let output = root.join("output");
    let report = extract_all(&archives, &output, false, ExecutionOptions::default()).unwrap();
    assert_eq!(report.discovered, 2);
    assert_eq!(report.extracted, 1);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(fs::read(output.join("good/member.txt")).unwrap(), b"good");
}

#[test]
fn add_files_to_archive_appends_new_members_and_replaces_with_overwrite() {
    let root = fixture("add-files-to-archive");
    let archive_path = root.join("sample.x");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("existing.txt", b"old")
        .add("keep.bin", b"keep")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, bytes).unwrap();

    let replacement = root.join("replacement.txt");
    let addition = root.join("addition.bin");
    fs::write(&replacement, b"new").unwrap();
    fs::write(&addition, b"added").unwrap();

    add_files_to_archive(
        &archive_path,
        &[
            ("existing.txt".into(), replacement.clone()),
            ("nested/addition.bin".into(), addition),
        ],
        AddFilesOptions {
            overwrite: true,
            ..AddFilesOptions::default()
        },
    )
    .unwrap();

    let rebuilt =
        sarges_unpacker::archive::Archive::parse(&fs::read(&archive_path).unwrap()).unwrap();
    assert_eq!(
        rebuilt.entries(),
        vec![
            "existing.txt".to_string(),
            "keep.bin".to_string(),
            "nested\\addition.bin".to_string()
        ]
    );
    let rebuilt_bytes = fs::read(&archive_path).unwrap();
    assert_eq!(
        rebuilt.payload(&rebuilt_bytes, "existing.txt").unwrap(),
        b"new"
    );
    assert_eq!(
        rebuilt
            .payload(&rebuilt_bytes, "nested\\addition.bin")
            .unwrap(),
        b"added"
    );
}

#[test]
fn add_files_to_archive_rejects_duplicates_without_modifying_archive() {
    let root = fixture("add-files-duplicate");
    let archive_path = root.join("sample.x");
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("existing.txt", b"old")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, &bytes).unwrap();
    let source = root.join("replacement.txt");
    fs::write(&source, b"new").unwrap();

    assert!(add_files_to_archive(
        &archive_path,
        &[("existing.txt".into(), source)],
        AddFilesOptions::default(),
    )
    .is_err());
    assert_eq!(fs::read(&archive_path).unwrap(), bytes);
}

#[test]
fn add_files_to_archive_imports_folders_and_creates_a_backup() {
    let root = fixture("add-files-folder");
    let archive_path = root.join("sample.x");
    let folder = root.join("selected");
    fs::create_dir_all(folder.join("nested")).unwrap();
    fs::write(folder.join("one.cfg"), b"one").unwrap();
    fs::write(folder.join("nested/two.cfg"), b"two").unwrap();
    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("existing.txt", b"old")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(&archive_path, &bytes).unwrap();
    let backup = root.join("sample-before.x");

    add_files_to_archive(
        &archive_path,
        &[],
        AddFilesOptions {
            folders: vec![folder],
            backup: Some(backup.clone()),
            ..AddFilesOptions::default()
        },
    )
    .unwrap();

    assert_eq!(fs::read(backup).unwrap(), bytes);
    let updated =
        sarges_unpacker::archive::Archive::parse(&fs::read(&archive_path).unwrap()).unwrap();
    assert_eq!(
        updated.entries(),
        vec![
            "existing.txt".to_string(),
            "nested\\two.cfg".to_string(),
            "one.cfg".to_string()
        ]
    );
}

#[test]
fn extract_all_preserves_relative_archive_directories() {
    let root = fixture("extract-all-relative-paths");
    let archives = root.join("archives");
    let nested = archives.join("missions").join("gb");
    fs::create_dir_all(&nested).unwrap();

    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("member.txt", b"nested")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(nested.join("attic.x"), bytes).unwrap();

    let mut bytes = Vec::new();
    sarges_unpacker::archive::ArchiveWriter::new()
        .add("member.txt", b"root")
        .write_to(&mut bytes)
        .unwrap();
    fs::write(archives.join("attic.x"), bytes).unwrap();

    let output = root.join("output");
    let report = extract_all(&archives, &output, true, ExecutionOptions::default()).unwrap();

    assert_eq!(report.discovered, 2);
    assert_eq!(report.extracted, 2);
    assert!(report.failures.is_empty());
    assert_eq!(fs::read(output.join("attic/member.txt")).unwrap(), b"root");
    assert_eq!(
        fs::read(output.join("missions/gb/attic/member.txt")).unwrap(),
        b"nested"
    );
}
