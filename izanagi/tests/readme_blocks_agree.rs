//! The workspace README's Rust blocks, checked against the engine README's —
//! which are compiled.
//!
//! The repository root README belongs to no crate, so nothing compiles it, and
//! it is the page GitHub shows first. The obvious fix was to include it as a
//! doctest from `izanagi/src/lib.rs`, and that was tried. It is wrong:
//! `include_str!("../../README.md")` reaches above the package directory, so
//! the file is not in the tarball and the published crate cannot run its own
//! doctests. Unpacking `izanagi-4.1.0.crate` and running `cargo test --doc`
//! fails on exactly that line.
//!
//! `cargo package --verify` does not catch it, which is worth stating plainly
//! because that step exists to catch missing files: it runs a *build*, and
//! `#[cfg(doctest)]` items do not exist during a build. The gate was green
//! with a broken published crate.
//!
//! So the blocks are checked a different way. The two READMEs carry the same
//! Rust, and the engine's copy *is* compiled as a doctest from inside the
//! package. Requiring the workspace README's blocks to appear verbatim in the
//! engine's therefore checks them without any file crossing a package
//! boundary — and enforces that the duplicate cannot drift, which was the
//! other half of the problem.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the engine lives one level below the workspace root")
        .to_path_buf()
}

/// The contents of every ```` ```rust ```` block in a markdown file.
fn rust_blocks(rel: &str) -> Vec<String> {
    let Ok(text) = fs::read_to_string(repo_root().join(rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some(tag) = line.strip_prefix("```") {
            match current.take() {
                Some(block) => out.push(block),
                None => {
                    if tag.trim() == "rust" {
                        current = Some(String::new());
                    }
                }
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    out
}

#[test]
fn every_workspace_readme_rust_block_also_appears_in_the_engine_readme() {
    let workspace = rust_blocks("README.md");
    let engine = rust_blocks("izanagi/README.md");

    // In a published tarball the workspace README is absent; there is then
    // nothing to check and nothing to complain about. In the repository it
    // must be there, which the assertion below distinguishes.
    if workspace.is_empty() && !repo_root().join("README.md").exists() {
        return;
    }

    assert!(
        !workspace.is_empty(),
        "README.md has no ```rust blocks — if the quickstart moved, this check \
         needs to move with it rather than silently pass"
    );
    assert!(
        !engine.is_empty(),
        "izanagi/README.md has no ```rust blocks, so there is nothing \
         compiled for the workspace README to agree with"
    );

    for (i, block) in workspace.iter().enumerate() {
        assert!(
            engine.contains(block),
            "README.md's Rust block #{} does not appear verbatim in \
             izanagi/README.md, so nothing compiles it. Either keep the two \
             copies identical, or move the block into izanagi/README.md and \
             link to it.\n\n--- the block ---\n{block}",
            i + 1
        );
    }
}

#[test]
fn no_doc_include_reaches_outside_its_package() {
    // The class, not just the instance. Any `include_str!` in library sources
    // whose path escapes the crate directory produces a crate that builds,
    // packages, and passes `cargo package --verify` — and then fails for the
    // first person who runs its doctests, because the file was never in the
    // tarball.
    for lib in ["izanagi/src/lib.rs", "izanagi_kit/src/lib.rs"] {
        let src = fs::read_to_string(repo_root().join(lib)).unwrap_or_else(|e| {
            panic!("reading {lib}: {e}");
        });
        let mut rest = src.as_str();
        while let Some(at) = rest.find("include_str!(\"") {
            rest = &rest[at + 14..];
            let Some(end) = rest.find('"') else { break };
            let path = &rest[..end];
            assert!(
                !path.starts_with("../../") && !path.contains("/../"),
                "{lib} includes `{path}`, which is outside the package. The \
                 file will not be in the published tarball, and `cargo package \
                 --verify` will not notice because it runs a build rather than \
                 the doctests."
            );
            rest = &rest[end..];
        }
    }
}

#[test]
fn the_block_extractor_reads_rust_and_skips_everything_else() {
    // Both directions. An extractor that returned nothing would make the
    // agreement test pass vacuously; one that swallowed prose would make it
    // fail on text that is not code.
    let engine = rust_blocks("izanagi/README.md");
    assert!(
        engine.iter().any(|b| b.contains("Engine::new()")),
        "the extractor did not find the engine README's quickstart"
    );
    assert!(
        !engine.iter().any(|b| b.contains("cargo test")),
        "a shell transcript was read as a Rust block — check the fence tags"
    );
    // A `text` fence between two `rust` fences must not merge them.
    let sample = "```rust\nlet a = 1;\n```\n```text\nnot rust\n```\n```rust\nlet b = 2;\n```\n";
    let path = repo_root().join("target/readme_blocks_agree_probe.md");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&path, sample).is_ok() {
        let blocks = rust_blocks("target/readme_blocks_agree_probe.md");
        assert_eq!(
            blocks,
            vec!["let a = 1;\n".to_string(), "let b = 2;\n".to_string()],
            "the extractor must take the two rust blocks and neither the text \
             block nor anything between them"
        );
        let _ = fs::remove_file(&path);
    }
}
