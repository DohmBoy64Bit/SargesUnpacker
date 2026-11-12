use sarges_unpacker::cli::{parse_args, Command, RuntimeOptions};

#[test]
fn parses_create_options() {
    let args = vec!["create", "-r", "-m=*.wav", "data.x", "src"];
    assert_eq!(
        parse_args(&args).unwrap(),
        Command::Create {
            archive: "data.x".into(),
            directory: Some("src".into()),
            recurse: true,
            mask: Some("*.wav".into()),
            options: RuntimeOptions::default()
        }
    );
}

#[test]
fn accepts_options_after_the_archive_and_directory() {
    let parsed = parse_args(&["create", "data.x", "src", "-r", "-m=*.TXT"]).unwrap();
    assert_eq!(
        parsed,
        Command::Create {
            archive: "data.x".into(),
            directory: Some("src".into()),
            recurse: true,
            mask: Some("*.TXT".into()),
            options: RuntimeOptions::default(),
        }
    );
}

#[test]
fn rejects_legacy_short_commands() {
    assert!(parse_args(&["c", "data.x"]).is_err());
    assert!(parse_args(&["x", "data.x"]).is_err());
    assert!(parse_args(&["l", "data.x"]).is_err());
}

#[test]
fn rejects_mask_without_an_equals_value() {
    assert!(parse_args(&["create", "-m", "data.x"]).is_err());
}

#[test]
fn parses_add_to_pack_member_source_pairs_and_overwrite() {
    let command = parse_args(&[
        "add-to-pack",
        "archive.x",
        "cfg/new.cfg=source/new.cfg",
        "textures/a.tga=C:\\tmp\\a.tga",
        "--overwrite",
    ])
    .unwrap();

    assert_eq!(
        command,
        Command::AddToPack {
            archive: "archive.x".into(),
            files: vec![
                ("cfg/new.cfg".into(), "source/new.cfg".into()),
                ("textures/a.tga".into(), "C:\\tmp\\a.tga".into()),
            ],
            overwrite: true,
            backup: None,
            folders: Vec::new(),
        }
    );
}

#[test]
fn rejects_invalid_add_to_pack_specification() {
    assert!(parse_args(&["add-to-pack", "archive.x", "missing-separator"]).is_err());
    assert!(parse_args(&["add-to-pack", "archive.x", "=source.bin"]).is_err());
    assert!(parse_args(&["add-to-pack", "archive.x", "member.bin="]).is_err());
    assert!(parse_args(&[
        "add-to-pack",
        "--skip-existing",
        "archive.x",
        "member.bin=source.bin"
    ])
    .is_err());
    assert!(parse_args(&[
        "add-to-pack",
        "--dry-run",
        "archive.x",
        "member.bin=source.bin"
    ])
    .is_err());
}

#[test]
fn parses_add_to_pack_folder_and_backup_options() {
    let command = parse_args(&[
        "add-to-pack",
        "--backup=before.x",
        "--folder=selected",
        "archive.x",
    ])
    .unwrap();
    assert_eq!(
        command,
        Command::AddToPack {
            archive: "archive.x".into(),
            files: Vec::new(),
            overwrite: false,
            backup: Some("before.x".into()),
            folders: vec!["selected".into()],
        }
    );
}

#[test]
fn parses_modern_operation_options() {
    let parsed = parse_args(&[
        "extract",
        "--json",
        "--dry-run",
        "--overwrite",
        "--progress",
        "data.x",
        "out",
    ])
    .unwrap();
    assert_eq!(
        parsed,
        Command::Extract {
            archive: "data.x".into(),
            directory: Some("out".into()),
            options: RuntimeOptions {
                output: sarges_unpacker::cli::OutputFormat::Json,
                dry_run: true,
                overwrite: sarges_unpacker::cli::OverwritePolicy::Overwrite,
                progress: true,
                quiet: false,
                verbose: false,
            },
        }
    );
}

#[test]
fn rejects_conflicting_overwrite_policies() {
    assert!(parse_args(&["extract", "--overwrite", "--skip-existing", "data.x"]).is_err());
}

#[test]
fn accepts_help_as_a_standalone_command() {
    assert_eq!(parse_args(&["--help"]).unwrap(), Command::Help);
}

#[test]
fn list_keeps_reference_behavior() {
    assert_eq!(
        parse_args(&["list", "data.x"]).unwrap(),
        Command::List {
            archive: "data.x".into(),
            options: RuntimeOptions::default()
        }
    );
}

#[test]
fn parses_extract_all_options() {
    let parsed = parse_args(&[
        "extract-all",
        "--recursive",
        "--dry-run",
        "archives",
        "output",
    ])
    .unwrap();

    assert_eq!(
        parsed,
        Command::ExtractAll {
            directory: "archives".into(),
            output: Some("output".into()),
            recurse: true,
            options: RuntimeOptions {
                dry_run: true,
                ..RuntimeOptions::default()
            },
        }
    );
}
