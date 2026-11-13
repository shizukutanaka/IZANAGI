//! IZANAGI Playground — discover, build and run the workspace's examples.
//!
//! The examples in `izanagi/examples/` and `izanagi_kit/examples/` are the
//! workspace's shop window: every one of them runs headless, prints a
//! result, and reproduces it byte for byte. They are normally exercised one
//! at a time through cargo or en masse by `tools/gate.sh`. This launcher
//! puts them on one menu so a person can browse and play them directly.
//!
//! The playground owns no demo logic of its own. The demo list is read from
//! the filesystem (the same source of truth the gate uses), the demos are
//! built with `cargo build --examples`, and each demo runs as the freshly
//! built binary in `target/debug/examples/`. A new example joins the menu
//! the day it is added, with no change here.
//!
//! Usage:
//!
//! ```text
//! izanagi-playground              interactive menu
//! izanagi-playground --list       print the demo list and exit
//! izanagi-playground --run pong   run one demo and exit
//! izanagi-playground --all        run every demo, print a summary, exit
//! ```
//!
//! Extra arguments for a demo are passed after `--`, for example
//! `izanagi-playground --run pong -- --terminal` plays pong in the
//! terminal. In the interactive menu, type a demo number to run it, a
//! number followed by arguments (for example `5 --terminal`) to pass
//! arguments through, `a` to run all demos, `l` to list again, or `q` to
//! quit.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

/// The crates whose `examples/` directories hold the demos, as
/// `(directory, label)` pairs, in menu order.
const CRATES: [(&str, &str); 2] = [("izanagi", "engine"), ("izanagi_kit", "kit")];

/// One runnable demo: an example source file in one of the crates.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Demo {
    /// Crate directory name, for example `izanagi_kit`.
    krate: String,
    /// Short label of the crate, for example `kit`.
    label: String,
    /// Example file stem, for example `wfc_demo`. This is also the name of
    /// the built binary under `target/debug/examples/`.
    name: String,
}

/// Finds the repository root: the nearest ancestor of `start` (or of the
/// running executable, as a fallback) that contains both crates' example
/// directories.
fn find_repo_root() -> Option<PathBuf> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            starts.push(dir.to_path_buf());
        }
    }
    for start in &starts {
        for ancestor in start.ancestors() {
            if CRATES
                .iter()
                .all(|(dir, _)| ancestor.join(dir).join("examples").is_dir())
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

/// Extracts the demo name from an example source path: the file stem of a
/// `.rs` file, or `None` for anything else (fixtures such as `.game`
/// files are content, not demos).
fn demo_name(path: &Path) -> Option<String> {
    if path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return None;
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
}

/// Reads the demo list from the filesystem under `root`, sorted by crate
/// and name, with duplicates removed.
fn discover_demos(root: &Path) -> Vec<Demo> {
    let mut demos: BTreeSet<Demo> = BTreeSet::new();
    for (dir, label) in CRATES {
        let examples = root.join(dir).join("examples");
        let Ok(entries) = fs::read_dir(&examples) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Some(name) = demo_name(&entry.path()) {
                demos.insert(Demo {
                    krate: dir.to_string(),
                    label: label.to_string(),
                    name,
                });
            }
        }
    }
    demos.into_iter().collect()
}

/// Builds every example in the workspace so the binaries under
/// `target/debug/examples/` are fresh. Cargo's own output goes to the
/// terminal: the first build is slow and the user should see why.
fn build_examples(root: &Path) -> io::Result<ExitStatus> {
    Command::new("cargo")
        .args(["build", "--workspace", "--examples"])
        .current_dir(root)
        .status()
}

/// The path of a demo's built binary.
fn demo_binary(root: &Path, demo: &Demo) -> PathBuf {
    root.join("target")
        .join("debug")
        .join("examples")
        .join(&demo.name)
}

/// Runs one demo attached to this terminal, passing `args` through, and
/// returns whether it exited successfully. Attached stdio is what makes
/// terminal-capable demos (pong with `--terminal`) actually playable, and
/// lets the text demos print straight to the user's screen.
fn run_attached(root: &Path, demo: &Demo, args: &[String]) -> io::Result<bool> {
    let status = Command::new(demo_binary(root, demo))
        .args(args)
        .current_dir(root)
        .status()?;
    Ok(status.success())
}

/// Runs one demo with its output captured, returning the exit success and
/// the combined output. Used by `--all`, which judges demos the way the
/// gate does: success, and something printed.
fn run_captured(root: &Path, demo: &Demo) -> io::Result<(bool, String)> {
    let out = Command::new(demo_binary(root, demo))
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), text))
}

/// Prints the demo menu.
fn print_menu(demos: &[Demo]) {
    println!("IZANAGI Playground — {} demos", demos.len());
    let mut current_label = "";
    for (i, demo) in demos.iter().enumerate() {
        if demo.label != current_label {
            current_label = &demo.label;
            println!("\n  [{}]", demo.label);
        }
        println!("  {:>3}. {:<28} ({})", i + 1, demo.name, demo.krate);
    }
    println!("\n  number: run a demo (extra words are passed as its arguments)");
    println!("  a: run all demos   l: list again   q: quit");
}

/// Parses one interactive menu line into an action.
#[derive(Debug, Eq, PartialEq)]
enum MenuAction {
    /// Run the demo with this 0-based index, passing these arguments.
    Run(usize, Vec<String>),
    /// Run every demo in captured mode.
    RunAll,
    /// Print the menu again.
    List,
    /// Leave the menu loop.
    Quit,
}

fn parse_menu_line(line: &str) -> Option<MenuAction> {
    let mut tokens = line.split_whitespace();
    let first = tokens.next()?;
    let args: Vec<String> = tokens.map(str::to_string).collect();
    match first {
        "q" | "quit" | "exit" => Some(MenuAction::Quit),
        "a" | "all" => Some(MenuAction::RunAll),
        "l" | "list" => Some(MenuAction::List),
        _ => first
            .parse::<usize>()
            .ok()
            .filter(|n| *n >= 1)
            .map(|n| MenuAction::Run(n - 1, args)),
    }
}

/// Runs every demo in captured mode and prints a summary line per demo,
/// mirroring the gate's example stage from the user's seat. Returns the
/// number of demos that failed.
fn run_all(root: &Path, demos: &[Demo]) -> usize {
    let mut failures = 0;
    for demo in demos {
        match run_captured(root, demo) {
            Ok((true, out)) if !out.trim().is_empty() => {
                let lines = out.lines().count();
                println!("  ok    {:<28} ({} lines printed)", demo.name, lines);
            }
            Ok((true, _)) => {
                failures += 1;
                println!("  FAIL  {:<28} printed nothing", demo.name);
            }
            Ok((false, out)) => {
                failures += 1;
                println!("  FAIL  {:<28} exited non-zero", demo.name);
                for line in out.lines().rev().take(5) {
                    println!("        {line}");
                }
            }
            Err(e) => {
                failures += 1;
                println!("  FAIL  {:<28} could not start: {e}", demo.name);
            }
        }
    }
    failures
}

/// Ensures the examples are built, once. `built` memoizes within a
/// session so menu runs do not pay the freshness check per demo — cargo
/// itself is the freshness check, and one call per session matches the
/// gate's behaviour.
fn ensure_built(root: &Path, built: &mut bool) -> bool {
    if *built {
        return true;
    }
    println!("building examples (cargo build --workspace --examples)...");
    match build_examples(root) {
        Ok(status) if status.success() => {
            *built = true;
            true
        }
        Ok(status) => {
            eprintln!("playground: example build failed ({status})");
            false
        }
        Err(e) => {
            eprintln!("playground: could not run cargo: {e}");
            false
        }
    }
}

fn interactive(root: &Path, demos: &[Demo]) -> i32 {
    print_menu(demos);
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut built = false;
    loop {
        print!("\nplayground> ");
        let _ = io::stdout().flush();
        let Some(Ok(line)) = lines.next() else {
            println!();
            return 0;
        };
        match parse_menu_line(&line) {
            Some(MenuAction::Quit) => return 0,
            Some(MenuAction::List) => print_menu(demos),
            Some(MenuAction::RunAll) => {
                if ensure_built(root, &mut built) {
                    let failures = run_all(root, demos);
                    println!(
                        "\n{} of {} demos ran clean",
                        demos.len() - failures,
                        demos.len()
                    );
                }
            }
            Some(MenuAction::Run(index, args)) => {
                let Some(demo) = demos.get(index) else {
                    println!("no demo number {}", index + 1);
                    continue;
                };
                if !ensure_built(root, &mut built) {
                    continue;
                }
                println!("\n--- {} ({}) ---", demo.name, demo.krate);
                match run_attached(root, demo, &args) {
                    Ok(true) => println!("--- {} finished ---", demo.name),
                    Ok(false) => println!("--- {} exited non-zero ---", demo.name),
                    Err(e) => eprintln!("playground: could not run {}: {e}", demo.name),
                }
            }
            None => println!("type a demo number, a, l, or q"),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(root) = find_repo_root() else {
        eprintln!(
            "playground: could not find the IZANAGI repository root \
             (a directory containing izanagi/examples and \
             izanagi_kit/examples) above the current directory"
        );
        std::process::exit(2);
    };
    let demos = discover_demos(&root);
    if demos.is_empty() {
        eprintln!("playground: no examples found under {}", root.display());
        std::process::exit(2);
    }

    let code = match args.first().map(String::as_str) {
        None => interactive(&root, &demos),
        Some("--list") => {
            print_menu(&demos);
            0
        }
        Some("--all") => {
            let mut built = false;
            if ensure_built(&root, &mut built) {
                let failures = run_all(&root, &demos);
                println!(
                    "\n{} of {} demos ran clean",
                    demos.len() - failures,
                    demos.len()
                );
                i32::from(failures > 0)
            } else {
                1
            }
        }
        Some("--run") => {
            let Some(name) = args.get(1) else {
                eprintln!("playground: --run needs a demo name (see --list)");
                std::process::exit(2);
            };
            let Some(demo) = demos.iter().find(|d| &d.name == name) else {
                eprintln!("playground: no demo named {name} (see --list)");
                std::process::exit(2);
            };
            // Everything after a literal `--` is passed to the demo.
            let extra: Vec<String> = match args.iter().position(|a| a == "--") {
                Some(pos) => args[pos + 1..].to_vec(),
                None => Vec::new(),
            };
            let mut built = false;
            if !ensure_built(&root, &mut built) {
                1
            } else {
                match run_attached(&root, demo, &extra) {
                    Ok(true) => 0,
                    Ok(false) => 1,
                    Err(e) => {
                        eprintln!("playground: could not run {}: {e}", demo.name);
                        1
                    }
                }
            }
        }
        Some(other) => {
            eprintln!("playground: unknown argument {other}");
            eprintln!("usage: izanagi-playground [--list | --all | --run <name> [-- <args>]]");
            2
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(krate: &str, label: &str, name: &str) -> Demo {
        Demo {
            krate: krate.to_string(),
            label: label.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn demo_name_takes_rust_file_stems_only() {
        assert_eq!(
            demo_name(Path::new("examples/wfc_demo.rs")).as_deref(),
            Some("wfc_demo")
        );
        assert_eq!(demo_name(Path::new("examples/dungeon.game")), None);
        assert_eq!(demo_name(Path::new("examples/README.md")), None);
    }

    #[test]
    fn menu_lines_parse_into_actions() {
        assert_eq!(parse_menu_line("q"), Some(MenuAction::Quit));
        assert_eq!(parse_menu_line("a"), Some(MenuAction::RunAll));
        assert_eq!(parse_menu_line("l"), Some(MenuAction::List));
        assert_eq!(
            parse_menu_line("5"),
            Some(MenuAction::Run(4, Vec::new())),
            "menu numbers are 1-based"
        );
        assert_eq!(
            parse_menu_line("5 --terminal"),
            Some(MenuAction::Run(4, vec!["--terminal".to_string()]))
        );
        assert_eq!(parse_menu_line("0"), None);
        assert_eq!(parse_menu_line("hello"), None);
        assert_eq!(parse_menu_line(""), None);
    }

    #[test]
    fn discovery_reads_both_crates_and_sorts() {
        // Fabricate a miniature repository in a scratch directory and
        // discover from it, so the test does not depend on the real
        // example set (which grows).
        let scratch =
            env::temp_dir().join(format!("izanagi-playground-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        for (dir, _, files) in [
            ("izanagi", "engine", &["world.rs", "pong.rs"][..]),
            ("izanagi_kit", "kit", &["wfc_demo.rs"][..]),
        ] {
            let examples = scratch.join(dir).join("examples");
            fs::create_dir_all(&examples).unwrap();
            for file in files {
                fs::write(examples.join(file), "fn main() {}\n").unwrap();
            }
        }
        // A fixture is not a demo and must not appear.
        fs::write(
            scratch.join("izanagi_kit/examples/dungeon.game"),
            "fixture\n",
        )
        .unwrap();

        let demos = discover_demos(&scratch);
        assert_eq!(
            demos,
            vec![
                demo("izanagi", "engine", "pong"),
                demo("izanagi", "engine", "world"),
                demo("izanagi_kit", "kit", "wfc_demo"),
            ]
        );
        let _ = fs::remove_dir_all(&scratch);
    }

    #[test]
    fn repo_root_is_found_above_a_nested_start() {
        let scratch =
            env::temp_dir().join(format!("izanagi-playground-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&scratch);
        for dir in ["izanagi", "izanagi_kit"] {
            fs::create_dir_all(scratch.join(dir).join("examples")).unwrap();
        }
        let nested = scratch.join("playground").join("src");
        fs::create_dir_all(&nested).unwrap();

        // Walk the same ancestor chain find_repo_root uses.
        let found = nested
            .ancestors()
            .find(|a| {
                CRATES
                    .iter()
                    .all(|(dir, _)| a.join(dir).join("examples").is_dir())
            })
            .map(Path::to_path_buf);
        assert_eq!(found, Some(scratch.clone()));
        let _ = fs::remove_dir_all(&scratch);
    }
}
