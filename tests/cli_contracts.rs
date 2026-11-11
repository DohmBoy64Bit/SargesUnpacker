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
