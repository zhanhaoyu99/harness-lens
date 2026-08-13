use std::{env, path::PathBuf};

use harness_lens_lib::{
    compatibility_report::{AggregateCompatibilityReport, ReportSource},
    scanner,
};

fn main() -> Result<(), String> {
    let Some(arguments) = parse_arguments(env::args().skip(1))? else {
        println!("{}", usage());
        return Ok(());
    };
    let home =
        dirs::home_dir().ok_or_else(|| "Unable to locate the home directory.".to_string())?;
    let snapshot = scanner::scan(&arguments.workspace, &home)?;
    let report = AggregateCompatibilityReport::from_snapshot(
        &snapshot,
        ReportSource::detect_from_build_checkout(),
    );

    if arguments.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
        );
    } else {
        println!("{}", report.to_markdown());
    }
    Ok(())
}

struct Arguments {
    workspace: PathBuf,
    json: bool,
}

fn parse_arguments(
    arguments: impl IntoIterator<Item = String>,
) -> Result<Option<Arguments>, String> {
    let mut json = false;
    let mut positional_only = false;
    let mut workspace = None;

    for argument in arguments {
        if !positional_only && argument == "--" {
            positional_only = true;
        } else if !positional_only && argument == "--json" {
            if json {
                return Err(format!("--json may be provided only once.\n{}", usage()));
            }
            json = true;
        } else if !positional_only && matches!(argument.as_str(), "--help" | "-h") {
            return Ok(None);
        } else if !positional_only && argument.starts_with('-') {
            return Err(format!("Unknown option: {argument}\n{}", usage()));
        } else if workspace.replace(PathBuf::from(&argument)).is_some() {
            return Err(format!("Only one workspace may be provided.\n{}", usage()));
        }
    }

    Ok(Some(Arguments {
        workspace: workspace.ok_or_else(|| usage().to_string())?,
        json,
    }))
}

fn usage() -> &'static str {
    "Usage: cargo run --example compatibility_report -- [--json] <workspace>"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_json_before_or_after_workspace() {
        for arguments in [vec!["--json", "/workspace"], vec!["/workspace", "--json"]] {
            let parsed = parse_arguments(arguments.into_iter().map(String::from))
                .expect("args")
                .expect("parsed arguments");
            assert!(parsed.json);
            assert_eq!(parsed.workspace, PathBuf::from("/workspace"));
        }
    }

    #[test]
    fn handles_help_and_rejects_unknown_flags_duplicates_and_extra_paths() {
        for arguments in [
            vec!["--unknown", "/workspace"],
            vec!["--json", "--json", "/workspace"],
            vec!["/workspace", "/other"],
            vec![],
        ] {
            assert!(parse_arguments(arguments.into_iter().map(String::from)).is_err());
        }
        assert!(parse_arguments(["--help"].into_iter().map(String::from))
            .expect("help")
            .is_none());
    }

    #[test]
    fn separator_allows_a_workspace_starting_with_a_dash() {
        let parsed = parse_arguments(["--", "-workspace"].into_iter().map(String::from))
            .expect("separator args")
            .expect("parsed arguments");
        assert_eq!(parsed.workspace, PathBuf::from("-workspace"));
        assert!(!parsed.json);
    }
}
