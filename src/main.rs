use sarges_unpacker::{
    cli::{self, Command},
    operations::{self, CreateOptions, ExecutionOptions, ExistingFilePolicy},
};
use std::env;
const VERSION: &str = "sarges-unpacker 0.1.0";
fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match cli::parse_args(&refs) {
        Ok(Command::Help) => {
            println!("{}", cli::usage());
            Ok(())
        }
        Ok(Command::Version) => {
            println!("{VERSION}");
            Ok(())
        }
        Ok(Command::List { archive, options }) => {
            if !options.quiet {
                if options.output == cli::OutputFormat::Json {
                    println!("{}", operations::list_json(&archive)?);
                } else {
                    print!("{}", operations::list(&archive)?);
                }
            }
            Ok(())
        }
        Ok(Command::Verify { archive, options }) => {
            if !options.quiet {
                if options.output == cli::OutputFormat::Json {
                    println!("{}", operations::verify_json(&archive)?);
                } else {
                    print!("{}", operations::verify(&archive)?);
                }
            }
            Ok(())
        }
        Ok(Command::Stats { archive, options }) => {
            if !options.quiet {
                if options.output == cli::OutputFormat::Json {
                    println!("{}", operations::stats_json(&archive)?);
                } else {
                    print!("{}", operations::stats(&archive)?);
                }
            }
            Ok(())
        }
        Ok(Command::Create {
            archive,
            directory,
            recurse,
            mask,
            options,
        }) => operations::create_with_options(
            directory.as_deref().unwrap_or(std::path::Path::new(".")),
            &archive,
            CreateOptions {
                recurse,
                mask: mask.as_deref(),
            },
            execution_options(&options),
        ),
        Ok(Command::Extract {
            archive,
            directory,
            options,
        }) => operations::extract_with_options(
            &archive,
            directory.as_deref().unwrap_or(std::path::Path::new(".")),
            execution_options(&options),
        ),
        Ok(Command::ExtractAll {
            directory,
            output,
            recurse,
            options,
        }) => {
            let report = operations::extract_all(
                &directory,
                output.as_deref().unwrap_or(&directory),
                recurse,
                execution_options(&options),
            )?;
            if report.failures.is_empty() {
                Ok(())
            } else {
                for failure in report.failures {
                    eprintln!("{failure}");
                }
                Err(std::io::Error::other(
                    "one or more archives failed to extract",
                ))
            }
        }
        Err(error) => {
            eprintln!("{}\n\n{}", error, cli::usage());
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, error))
        }
    }
}
fn execution_options(o: &cli::RuntimeOptions) -> ExecutionOptions {
    ExecutionOptions {
        dry_run: o.dry_run,
        existing: match o.overwrite {
            cli::OverwritePolicy::Refuse => ExistingFilePolicy::Refuse,
            cli::OverwritePolicy::Overwrite => ExistingFilePolicy::Overwrite,
            cli::OverwritePolicy::Skip => ExistingFilePolicy::Skip,
        },
        progress: o.progress && !o.quiet,
    }
}
