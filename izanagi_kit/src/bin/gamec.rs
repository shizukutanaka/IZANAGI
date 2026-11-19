//! `gamec` — the game-content checker/compiler.
//!
//! Usage: `gamec [--help | --fmt | --json | --sarif | --check | --rich | --verify] <file.game>`
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
//! `--fmt` writes canonical serialized content to stdout; the input file is
//! not modified. Canonical form sorts `stat` keys, uppercases hex colors and
//! drops comments and blank lines — pipe it back over the file to format it.
//! `--check` verifies formatting without output — exits non-zero if any
//! content line differs from the canonical serialized form. `//` comments and
//! blank lines are free-form: the canonical form carries none, so they are
//! ignored by the comparison (like `cargo fmt --check` tolerating comments).
//! `--rich` keeps the human-readable flow and exit codes, but renders each
//! diagnostic through the span-based renderer ([`izanagi_kit::diag_render`]):
//! a `--> file:line:col` locator, a numbered source block and an underlined
//! span instead of the one-line form.
//!
//! `--verify` runs the bundle determinism audit from the `bundle_verify`
//! module: every level is loaded
//! twice, soaked with a canonical seeded workload, and checked for double-run
//! and rollback-resimulation divergence. On divergence the report names the
//! first diverging tick — a report, not a panic. Exit codes follow the other
//! modes: `0` when every level is deterministic, `1` when validation fails or
//! any level diverges, `2` for usage errors and unreadable input — so the
//! check drops straight into CI as a determinism gate.

use izanagi_kit::bundle_verify::{verify_bundle, DEFAULT_TICKS};
use izanagi_kit::content::{Diagnostic, Severity};
use izanagi_kit::diag_json::{diag_json, diag_sarif};
use izanagi_kit::diag_render::RichDiagnostic;
use izanagi_kit::{is_loadable, load_level, parse, validate};
use std::io::Write;
use std::process::ExitCode;

const USAGE: &str =
    "usage: gamec [--help | --fmt | --json | --sarif | --check | --rich | --verify] <file.game>";

/// Render one diagnostic in the form the selected mode asks for: the
/// span-based rich form under `--rich`, the one-line form otherwise.
fn render_diag(mode: &OutputMode, d: &Diagnostic, path: &str, source: &str) -> String {
    if *mode == OutputMode::Rich {
        let mut rich = RichDiagnostic::from_diagnostic(d);
        if let Some((message, suggestion)) = d
            .message
            .strip_suffix("?)")
            .and_then(|message| message.rsplit_once(" (did you mean '"))
        {
            if let Some(name) = suggestion.strip_suffix('\'') {
                rich.message = message.to_string();
                rich = rich.with_help(format!("did you mean '{name}'?"));
            }
        }
        rich.render(path, source)
    } else {
        d.render(path, source)
    }
}

/// Writes payload output through a locked stdout so a closed pipe (EPIPE,
/// e.g. `gamec --json | head -1`) is a clean failure, not a panic.
fn emit(text: &str) -> bool {
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes()).is_ok() && out.flush().is_ok()
}

#[derive(PartialEq)]
enum OutputMode {
    Human,
    Fmt,
    Json,
    Sarif,
    Check,
    Rich,
    Verify,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [flag, p] if flag == "--fmt" => (OutputMode::Fmt, p.clone()),
        [flag, p] if flag == "--json" => (OutputMode::Json, p.clone()),
        [flag, p] if flag == "--sarif" => (OutputMode::Sarif, p.clone()),
        [flag, p] if flag == "--check" => (OutputMode::Check, p.clone()),
        [flag, p] if flag == "--rich" => (OutputMode::Rich, p.clone()),
        [flag, p] if flag == "--verify" => (OutputMode::Verify, p.clone()),
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        [p] if !p.starts_with('-') => (OutputMode::Human, p.clone()),
        _ => {
            eprintln!("{USAGE}");
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
        let ok = is_loadable(&parse_diags, &validate_diags);
        if !emit(&format!("{}\n", diag_json(&path, &all_diags))) {
            eprintln!("{path}: error: failed to write JSON to stdout");
            return ExitCode::FAILURE;
        }
        return if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    if mode == OutputMode::Sarif {
        let ok = is_loadable(&parse_diags, &validate_diags);
        if !emit(&format!("{}\n", diag_sarif(&path, &all_diags))) {
            eprintln!("{path}: error: failed to write SARIF to stdout");
            return ExitCode::FAILURE;
        }
        return if ok {
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
        // Trivia (`//` comment lines and blank lines) carry no semantics and
        // the canonical form emits none — compare content lines only, so a
        // commented, sectioned file can still pass the format gate.
        let trivia = |l: &&str| {
            let t = l.trim_start();
            t.is_empty() || t.starts_with("//")
        };
        if source.lines().filter(|l| !trivia(l)).ne(canonical.lines()) {
            eprintln!("{path}: file needs formatting (content differs when serialized)");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    if mode == OutputMode::Verify {
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
            eprintln!("FAILED: {errors} error(s), {warnings} warning(s)");
            return ExitCode::FAILURE;
        }
        return match verify_bundle(&content, DEFAULT_TICKS) {
            Ok(report) => {
                let deterministic = report.is_deterministic();
                if !emit(&format!("{report}\n")) {
                    eprintln!("{path}: error: failed to write verification report to stdout");
                    return ExitCode::FAILURE;
                }
                if deterministic {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("{path}: error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let mut errors = 0usize;
    let mut warnings = 0usize;
    for d in &all_diags {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
        }
        eprintln!("{}", render_diag(&mode, d, &path, &source));
    }

    if !is_loadable(&parse_diags, &validate_diags) {
        if mode != OutputMode::Fmt {
            emit(&format!(
                "parsed: {} prefab(s), {} tile(s), {} level(s)\n",
                content.prefabs.len(),
                content.tiles.len(),
                content.levels.len()
            ));
        }
        eprintln!("FAILED: {errors} error(s), {warnings} warning(s)");
        return ExitCode::FAILURE;
    }

    if mode == OutputMode::Fmt {
        // Emit canonical text to stdout (pipeable); diagnostics already on stderr.
        if !emit(&izanagi_kit::serialize(&content)) {
            eprintln!("{path}: error: failed to write formatted content to stdout");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    let mut out = format!(
        "parsed: {} prefab(s), {} tile(s), {} level(s)\n",
        content.prefabs.len(),
        content.tiles.len(),
        content.levels.len()
    );

    for level in &content.levels {
        match load_level(&content, &level.name) {
            Ok(w) => out.push_str(&format!(
                "level '{}': {} entit(y/ies) loaded\n",
                level.name,
                w.entity_count()
            )),
            Err(e) => {
                eprintln!("{path}: error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    out.push_str(&format!("OK: {warnings} warning(s)\n"));
    if !emit(&out) {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPAWN: &str = "prefab ghost\nglyph g\nlevel a 1x1\nrow .\nspawn ghast 0 0\n";
    const EXTENDS: &str = "prefab enemy\nglyph e\nprefab boss extends enem\nglyph b\n";
    const NO_SUGGESTION: &str = "prefab goblin\nglyph g\nlevel a 1x1\nrow .\nspawn zz 0 0\n";

    fn validation_error(source: &str) -> Diagnostic {
        let (content, parse_diags) = parse(source);
        assert!(parse_diags.is_empty(), "{parse_diags:?}");
        let errors: Vec<_> = validate(&content)
            .into_iter()
            .filter(|d| d.is_error())
            .collect();
        assert_eq!(errors.len(), 1, "{errors:?}");
        errors[0].clone()
    }

    #[test]
    fn rich_spawn_suggestion_is_a_help_line() {
        let diagnostic = validation_error(SPAWN);
        let expected = "error: level 'a': spawn references undefined prefab 'ghast'\n  = help: did you mean 'ghost'?";
        for _ in 0..2 {
            assert_eq!(
                render_diag(&OutputMode::Rich, &diagnostic, "test.game", SPAWN),
                expected
            );
        }
    }

    #[test]
    fn rich_extends_suggestion_is_a_help_line() {
        let diagnostic = validation_error(EXTENDS);
        let expected =
            "error: prefab 'boss' extends undefined prefab 'enem'\n  = help: did you mean 'enemy'?";
        assert_eq!(
            render_diag(&OutputMode::Rich, &diagnostic, "test.game", EXTENDS),
            expected
        );
    }

    #[test]
    fn rich_without_a_suggestion_is_unchanged() {
        let diagnostic = validation_error(NO_SUGGESTION);
        let expected = "error: level 'a': spawn references undefined prefab 'zz'";
        assert_eq!(
            render_diag(&OutputMode::Rich, &diagnostic, "test.game", NO_SUGGESTION),
            expected
        );
        assert_eq!(
            RichDiagnostic::from_diagnostic(&diagnostic).render("test.game", NO_SUGGESTION),
            expected
        );
    }

    #[test]
    fn plain_suggestions_and_other_formats_remain_unchanged() {
        for (source, message) in [
            (
                SPAWN,
                "level 'a': spawn references undefined prefab 'ghast' (did you mean 'ghost'?)",
            ),
            (
                EXTENDS,
                "prefab 'boss' extends undefined prefab 'enem' (did you mean 'enemy'?)",
            ),
            (
                NO_SUGGESTION,
                "level 'a': spawn references undefined prefab 'zz'",
            ),
        ] {
            let diagnostic = validation_error(source);
            let legacy = Diagnostic::error(0, message);
            let expected = format!("test.game: error: {message}");
            assert_eq!(
                render_diag(&OutputMode::Human, &diagnostic, "test.game", source),
                expected
            );
            let _ = render_diag(&OutputMode::Rich, &diagnostic, "test.game", source);
            assert_eq!(diagnostic.message, message);
            assert_eq!(diagnostic.severity, Severity::Error);
            assert_eq!(
                diag_json("test.game", std::slice::from_ref(&diagnostic)),
                diag_json("test.game", std::slice::from_ref(&legacy))
            );
            assert_eq!(
                diag_sarif("test.game", &[diagnostic]),
                diag_sarif("test.game", &[legacy])
            );
        }
    }

    #[test]
    fn rich_empty_and_incomplete_suggestions_are_unchanged() {
        for message in [
            "",
            "did you mean",
            "example (did you mean 'ghost'?",
            "example (did you mean ghost?)",
            "example (did you mean 'ghost'?) trailing",
        ] {
            for diagnostic in [
                Diagnostic::error(0, message),
                Diagnostic::warning(0, message),
            ] {
                assert_eq!(
                    render_diag(&OutputMode::Rich, &diagnostic, "test.game", ""),
                    RichDiagnostic::from_diagnostic(&diagnostic).render("test.game", "")
                );
            }
        }
    }
}
