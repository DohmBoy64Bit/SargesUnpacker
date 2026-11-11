//! Command-line argument parsing and runtime option types.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Selects human-readable or JSON command output.
pub enum OutputFormat {
    /// Human-readable text output.
    Human,
    /// Machine-readable JSON output.
    Json,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Controls behavior when extraction encounters an existing file.
pub enum OverwritePolicy {
    /// Fail if the destination already exists.
    Refuse,
    /// Replace an existing destination.
    Overwrite,
    /// Leave an existing destination unchanged.
    Skip,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Options shared by archive operations.
pub struct RuntimeOptions {
    /// Selected output representation.
    pub output: OutputFormat,
    /// Validate or plan without writing files.
    pub dry_run: bool,
    /// Existing-file behavior for extraction.
    pub overwrite: OverwritePolicy,
    /// Emit per-member progress messages.
    pub progress: bool,
    /// Suppress normal operation output.
    pub quiet: bool,
    /// Enable verbose operation output.
    pub verbose: bool,
}
impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            output: OutputFormat::Human,
            dry_run: false,
            overwrite: OverwritePolicy::Refuse,
            progress: false,
            quiet: false,
            verbose: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed command-line request.
pub enum Command {
    /// Creates an archive from a directory.
    Create {
        /// Destination archive path.
        archive: PathBuf,
        /// Source directory, defaulting to the current directory.
        directory: Option<PathBuf>,
        /// Include nested directories.
        recurse: bool,
        /// Optional member-name mask.
        mask: Option<String>,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Extracts an archive into a directory.
    Extract {
        /// Source archive path.
        archive: PathBuf,
        /// Destination directory, defaulting to the current directory.
        directory: Option<PathBuf>,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Extracts every `.x` archive found in a directory.
    ExtractAll {
        /// Directory in which archives are discovered.
        directory: PathBuf,
        /// Root for per-archive output directories.
        output: Option<PathBuf>,
        /// Include archives in nested directories.
        recurse: bool,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Lists archive members.
    List {
        /// Source archive path.
        archive: PathBuf,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Validates an archive.
    Verify {
        /// Source archive path.
        archive: PathBuf,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Reports archive statistics.
    Stats {
        /// Source archive path.
        archive: PathBuf,
        /// Shared runtime options.
        options: RuntimeOptions,
    },
    /// Prints the program version.
    Version,
    /// Prints command usage.
    Help,
}
/// Parses command-line arguments after the executable name.
pub fn parse_args(args: &[&str]) -> Result<Command, String> {
    if args.is_empty() || matches!(args[0], "--help" | "-h") {
        return Ok(Command::Help);
    }
    if matches!(args[0], "--version" | "-V") {
        return Ok(Command::Version);
    }
    let command = args[0];
    if !matches!(
        command,
        "create" | "extract" | "extract-all" | "list" | "verify" | "stats"
    ) {
        return Err(format!("unknown command: {command}"));
    }
    let mut recurse = false;
    let mut mask = None;
    let mut options = RuntimeOptions::default();
    let mut positional = Vec::new();
    for option in &args[1..] {
        if let Some(name) = option
            .strip_prefix("--")
            .or_else(|| option.strip_prefix('-'))
        {
            match name {
                "r" | "recursive" => recurse = true,
                value if value.starts_with("m=") && value.len() > 2 => {
                    mask = Some(value[2..].to_owned())
                }
                "m" => return Err("file mask requires a value".into()),
                "json" => options.output = OutputFormat::Json,
                "dry-run" => options.dry_run = true,
                "overwrite" if options.overwrite == OverwritePolicy::Skip => {
                    return Err("overwrite policies conflict".into())
                }
                "overwrite" => options.overwrite = OverwritePolicy::Overwrite,
                "skip-existing" if options.overwrite == OverwritePolicy::Overwrite => {
                    return Err("overwrite policies conflict".into())
                }
                "skip-existing" => options.overwrite = OverwritePolicy::Skip,
                "progress" => options.progress = true,
                "quiet" => options.quiet = true,
                "verbose" => options.verbose = true,
                "help" => return Ok(Command::Help),
                _ => return Err(format!("unknown option: {option}")),
            }
        } else {
            positional.push(*option);
        }
    }
    let archive = positional.first().ok_or("missing archive path")?.into();
    let directory = positional.get(1).map(PathBuf::from);
    if positional.len() > 2 {
        return Err("too many positional arguments".into());
    }
    Ok(match command {
        "create" => Command::Create {
            archive,
            directory,
            recurse,
            mask,
            options,
        },
        "extract" => Command::Extract {
            archive,
            directory,
            options,
        },
        "extract-all" => Command::ExtractAll {
            directory: archive,
            output: directory,
            recurse,
            options,
        },
        "list" => Command::List { archive, options },
        "verify" => Command::Verify { archive, options },
        "stats" => Command::Stats { archive, options },
        _ => unreachable!(),
    })
}
/// Returns the command-line usage text.
pub fn usage() -> &'static str {
    "Sarges Unpacker\n\nUsage: sarges-unpacker <command> [options] <archive> [directory]\n\nCommands: create, extract, extract-all, list, verify, stats, --version\nOptions: -r, --recursive, -m=MASK, --json, --dry-run, --overwrite, --skip-existing, --progress, --quiet, --verbose"
}
