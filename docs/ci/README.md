# Enabling CI

This repository has **no active CI**: the 3,700+ tests, the determinism
matrix, and the packaging checks run only on developer machines. The workflow
definition is ready — it just cannot be installed from here.

## Why the agent cannot do this step

Agent sessions authenticate with a GitHub App token that lacks the `workflows`
permission. **Both available paths were tested, and both are blocked:**

| Path | Result |
| --- | --- |
| `git push` touching `.github/workflows/` | the entire push is rejected |
| Contents API (`PUT /repos/{o}/{r}/contents/...`) | `403 Resource not accessible by integration` |
| Git Data API (`POST /git/trees`, i.e. a multi-file push) | `403 Resource not accessible by integration` |

So this is a measured limitation, not an assumption inherited from a comment —
there is no agent-side workaround, and the file has to be created by someone
authenticating as a human. That is why the definition sits in `docs/ci/ci.yml`
instead of its real home.

## What a maintainer does (once, ~2 minutes)

1. Open the repository on github.com → **Add file → Create new file**.
2. Name it exactly `.github/workflows/ci.yml`.
3. Paste the contents of [`docs/ci/ci.yml`](./ci.yml) (drop the explanatory
   comment block at the top if you like — it is about *this* copy).
4. Commit. Actions starts running on the next push.

Alternatively, from any clone that pushes with your own credentials:

```text
mkdir -p .github/workflows
cp docs/ci/ci.yml .github/workflows/ci.yml
git add .github/workflows/ci.yml
git commit -m "Enable CI"
git push
```

## What the workflow runs

| Job | What it proves |
| --- | --- |
| `gate` | `tools/gate.sh` — the same script the pre-push hook runs: fmt, workspace tests, clippy at zero warnings, rustdoc at zero warnings, the pinned determinism hashes, every example run twice and required to complete headless, print a result and reproduce it byte for byte, the `kit_bridge` integration hash, the self-asserting pipeline demo, `cargo package` for both crates, and `cargo test --doc`, `cargo test --tests` and `cargo build --examples` all run *inside* each unpacked tarball — a build alone cannot see a `#[cfg(doctest)]` item, so it cannot see a doc include pointing outside the package. One definition of "green", shared between local and CI. |
| `msrv` | The declared MSRVs are real: `izanagi` builds on 1.65, `izanagi_kit` on 1.75 (`cargo check`, since the MSRV promise is to consumers, not to the test suite). |
| `wasm` | `izanagi_kit` compiles for `wasm32-unknown-unknown`. |

Keep `docs/ci/ci.yml` and the installed `.github/workflows/ci.yml` in sync:
propose changes here, apply them there.
