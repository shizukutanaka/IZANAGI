//! `gamec` — the game-content checker/compiler.
//!
//! Usage: `gamec <file.game>`
//!
//! Parses and validates an authored content file, prints every diagnostic with
//! its line number, and on success prints a load summary (entity counts per
//! level). Exits non-zero if any error-severity diagnostic is found, so it
//! drops straight into CI as a content gate.

use izanagi_kit::content::Severity;
use izanagi_kit::{is_loadable, load_level, parse, validate};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (fmt, path) = match args.as_slice() {
        [flag, p] if flag == "--fmt" => (true, p.clone()),
        [p] => (false, p.clone()),
        _ => {
            eprintln!("usage: gamec [--fmt] <file.game>");
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

    let mut errors = 0usize;
    let mut warnings = 0usize;
    for d in parse_diags.iter().chain(validate_diags.iter()) {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
        eprintln!("{}", d.render(&path, &source));
    }

    if !is_loadable(&parse_diags, &validate_diags) {
        if !fmt {
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

    if fmt {
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
