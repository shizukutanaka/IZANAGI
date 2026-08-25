//! `gamec` — the game-content checker/compiler.
//!
//! Usage: `gamec [--fmt | --json | --sarif | --check] <file.game>`
//!
//! Parses and validates an authored content file, prints every diagnostic with
//! its line number, and on success prints a load summary (entity counts per
//! level). Exits non-zero if any error-severity diagnostic is found, so it
//! drops straight into CI as a content gate.
//!
//! `--json` emits all diagnostics as a JSON object to stdout (machine-readable;
//! satisfies taxonomy P4). Human-readable diagnostics are suppressed on stderr.
//! `--sarif` emits diagnostics as a SARIF 2.1.0 document, the format GitHub
//! Code Scanning's `upload-sarif` action consumes for inline PR annotations.
//! `--fmt` emits canonical serialized content to stdout (no change).
//! `--check` verifies formatting without output — exits non-zero if the file's
//! serialized form differs from its source (like `cargo fmt --check`).

use izanagi_kit::content::Severity;
use izanagi_kit::diag_json::{diag_json, diag_sarif};
use izanagi_kit::{is_loadable, load_level, parse, validate};
use std::process::ExitCode;

#[derive(PartialEq)]
enum OutputMode {
    Human,
    Fmt,
    Json,
    Sarif,
    Check,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [flag, p] if flag == "--fmt" => (OutputMode::Fmt, p.clone()),
        [flag, p] if flag == "--json" => (OutputMode::Json, p.clone()),
        [flag, p] if flag == "--sarif" => (OutputMode::Sarif, p.clone()),
        [flag, p] if flag == "--check" => (OutputMode::Check, p.clone()),
        [p] => (OutputMode::Human, p.clone()),
        _ => {
            eprintln!("usage: gamec [--fmt | --json | --sarif | --check] <file.game>");
            return ExitCode::from(2);
        }
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };

    let (content, parse_diags) = parse(&source);
    let validate_diags = validate(&content);
    let all_diags: Vec<_> = parse_diags
        .iter()
        .chain(validate_diags.iter())
        .cloned()
        .collect();

    if mode == OutputMode::Json {
        println!("{}", diag_json(&path, &all_diags));
        return if is_loadable(&parse_diags, &validate_diags) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if mode == OutputMode::Sarif {
        println!("{}", diag_sarif(&path, &all_diags));
        return if is_loadable(&parse_diags, &validate_diags) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if mode == OutputMode::Check {
        if !is_loadable(&parse_diags, &validate_diags) {
            let mut errors = 0usize;
            let mut warnings = 0usize;
            for d in &all_diags {
                match d.severity {
                    Severity::Error => errors += 1,
                    Severity::Warning => warnings += 1,
                }
                eprintln!("{}", d.render(&path, &source));
            }
            eprintln!("FAILED: {errors} error(s), {warnings} warning(s)");
            return ExitCode::FAILURE;
        }
        let canonical = izanagi_kit::serialize(&content);
        if source != canonical {
            eprintln!(
                "{}: file needs formatting (content differs when serialized)",
                path
            );
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    let mut errors = 0usize;
    let mut warnings = 0usize;
    for d in &all_diags {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
        eprintln!("{}", d.render(&path, &source));
    }

    if !is_loadable(&parse_diags, &validate_diags) {
        if mode != OutputMode::Fmt {
            println!(
                "parsed: {} prefab(s), {} tile(s), {} level(s)",
                content.prefabs.len(),
                content.tiles.len(),
                content.levels.len()
            );
        }
        eprintln!("FAILED: {errors} error(s), {warnings} warning(s)");
        return ExitCode::FAILURE;
    }

    if mode == OutputMode::Fmt {
        // Emit canonical text to stdout (pipeable); diagnostics already on stderr.
        print!("{}", izanagi_kit::serialize(&content));
        return ExitCode::SUCCESS;
    }

    println!(
        "parsed: {} prefab(s), {} tile(s), {} level(s)",
        content.prefabs.len(),
        content.tiles.len(),
        content.levels.len()
    );

    for level in &content.levels {
        match load_level(&content, &level.name) {
            Ok(w) => println!(
                "level '{}': {} entit(y/ies) loaded",
                level.name,
                w.entity_count()
            ),
            Err(e) => {
                eprintln!("{path}: error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    println!("OK: {warnings} warning(s)");
    ExitCode::SUCCESS
}
