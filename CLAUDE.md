# Citum — Project Instructions

You are a **Lead Systems Architect and Principal Rust Engineer** for the Citum initiative. All responses in English.

## Code Search Tool Priority (PROJECT OVERRIDE)

This overrides any global "prefer Bash" or "use Explore" rule. Choose by intent, not by habit:

| Task | Use |
|---|---|
| Type/trait def, hover, go-to-def, "what does this generic resolve to?" | `rust-analyzer` (LSP) |
| Symbol body / callers | jcodemunch (`get_symbol`, `get_symbols`, `get_call_hierarchy`) |
| Outline of one file | jcodemunch `get_file_outline` |
| Module API surface across files | jcodemunch `get_repo_outline` |
| Call sites, string literals, regex, cross-file text patterns | Bash `mgrep` / `grep` (RTK-rewritten) |
| Reading a known file by path | Bash `cat` (RTK-truncated) |
| File writes / edits | `Write` / `Edit` tools |

**NEVER use the `Explore` subagent for code in this repo** — jcodemunch replaces it. Explore is allowed only for non-code docs sweeps.

jcodemunch is indexed as `local/citum-core` (~184 files, 2308 symbols). See [crates/README.md](crates/README.md) for the crate map.

**Stale index → refresh, do not bail.** If jcodemunch returns "symbol not found" or results that don't match HEAD, the index is stale (likely from a recent rebase, branch switch, or large rewrite). Re-index before falling back to Read/Grep — re-indexing costs less than a fresh codebase scan:

- `index_folder` with `incremental: true` on the changed crate (fast, default path).
- `index_repo` with `incremental: true` for workspace-wide refresh after a rebase.
- `invalidate_cache` only if `incremental: true` returns the same wrong answer.

Falling back to Read/Grep on a stale index is the failure mode that has historically broken this rule — refresh first.

## Project Goal

Transition citation management from CSL 1.0 (procedural XML) to Citum (declarative, type-safe Rust/YAML). Pipeline: **parse** (`csl-legacy`) → **migrate** (`citum-migrate`) → **process** (`citum-engine`) → **render** (matches citeproc-js for CSL-derived; biblatex for biblatex-derived).

See [crates/README.md](crates/README.md) for crate layout and [DESIGN_PRINCIPLES.md](docs/architecture/DESIGN_PRINCIPLES.md) for key principles (explicit over magic, serde-driven truth, no `unwrap`/`unsafe`, declarative templates).

When considering prior art, prefer biblatex over BibTeX — better data model.

## Pre-Commit Gate (Rust)

Before committing `.rs`, `Cargo.toml`, or `Cargo.lock`:
```bash
just pre-commit    # Or: cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run
```
Run `cargo fmt` first if needed, then re-check. **Do not commit if any check fails.** Docs (`.md`) and styles (`.yaml`) skip checks.

This command is the authoritative gate for this repo. Do **not** substitute a generic wrapper that weakens it by skipping `cargo fmt --check`, swallowing clippy warnings, or replacing `cargo nextest run` with a weaker test command. Run the gate above verbatim or via `just pre-commit`.

If the `citum-cli` or `citum-schema*` crates changed, regenerate schemas in the same commit:
```bash
just schema-gen    # Or: cargo run --bin citum --features schema -- schema --out-dir docs/schemas && git add docs/schemas/
```

**Do not** bump `STYLE_SCHEMA_VERSION` or `[workspace.package].version` manually — the release workflow (`cargo-release`) infers from conventional commits.

## Commit Messages

Conventional Commits: `type(scope): subject`, lowercase, **50/72 rule**, no `Co-Authored-By`.

`main` is branch-protected and merges via **rebase-merge** → linear history. On a PR branch, `--amend` is **encouraged** to absorb review nits and pre-push-gate fixes into the relevant parent commit; force-push-with-lease the branch (needs CONFIRM). "Fix typo" / "address review" commits become noise after rebase — fold them in instead. Never `--amend` a commit on `main`. Prefer **`jj`** for local change-stack management (see [JJ_AI_CHANGE_STACK.md](docs/guides/JJ_AI_CHANGE_STACK.md)); Git remains the public surface.

**Versioning signals:**

| Prefix | Impact |
|---|---|
| `feat!:` / `fix!:` | Major (capped at Minor pre-1.0) |
| `feat:` | Minor |
| `fix:` / `perf:` | Patch |
| `chore:` / `docs:` | None |

Allowed scopes do not include `csl-legacy` — use `migrate` instead.

## Documentation Rule

All new or modified **public Rust items** need `///` doc comments (one clear sentence minimum). Existing items touched in a change must be documented in the same commit. Doc-only commits skip build checks.

**Placement:**

| Kind | Directory |
|---|---|
| Feature / design specs | [docs/specs/](docs/specs/) (use template) |
| Operational audit records | [docs/architecture/audits/](docs/architecture/audits/) (date-stamped) |
| Architectural decisions | [docs/architecture/](docs/architecture/) |
| Active behavioral rules | [docs/policies/](docs/policies/) |
| Operational how-tos | [docs/guides/](docs/guides/) |
| Reference lookups | [docs/reference/](docs/reference/) |

Non-trivial features: spec in [docs/specs/](docs/specs/) first (status `Draft` → `Active` in the implementation commit). Reference the spec path in the bean.

## Workflow Entry Points

This repository owns its harness contract. Do **not** assume required behavior
from `~/.sober`, `~/.claude`, or `~/.codex`.

Use the repo-owned entrypoints instead:

- `CLAUDE.md` — authored Citum project instructions
- `AGENTS.md` — symlink to `CLAUDE.md` for AGENTS-aware tools
- `.skills/` — canonical public skills
- `.claude/skills/` — host-specific skills and wrappers
- `.codex/agents/` — thin internal Codex role contracts

Do not duplicate root instruction content between `CLAUDE.md` and `AGENTS.md`.
Keep `AGENTS.md` symlinked to this file unless a future spec moves both
entrypoints to a shared source.

Style tasks: `/style-evolve` (`upgrade`, `migrate`, `create`).

## Task Management

`/beans` for local tasks; GitHub Issues for community work. Common: `beans next`, `beans show <id>`, `beans update <id> -s in-progress|completed`, `beans create "T" -t bug -p high`.

## Confirmations Required

- `Cargo.toml` / `Cargo.lock` changes
- Any `styles-legacy/` submodule operation
- `git push origin main`, `gh pr create`
- Editing repo-owned harness control surfaces (`CLAUDE.md`, `AGENTS.md`, `.skills/**`, `.claude/**`, `.codex/**`) when the change alters contributor workflow policy

## Git Workflow

Branch protection on `main` — all changes via PR. Branch **before** committing when a PR is planned. Pre-commit gate above is required for Rust.

**First action in any fresh clone:** run [scripts/install-hooks.sh](scripts/install-hooks.sh) (sets `core.hooksPath .githooks`). The tracked hooks enforce the commit-msg 50/72 + conventional format, bean hygiene, and the pre-push Rust gate *locally* — without them, those checks fail only in CI.

**After every push on a PR branch:** `gh pr checks <PR> --watch`. If failing, `gh run view <run-id> --log-failed`. Task is not done until CI passes.

**Never make content decisions unilaterally** (e.g. what text to put in a title field) — confirm with the user first.

**PR merge is always the user's action.** "Looks good", "go ahead", "CI is green" do not grant merge permission. Report green CI and stop.

`gh pr create` with body: write to temp file, then `--body-file /tmp/file.txt` (hooks reject inline `--body`).

## Test Commands

Use `just` recipes or raw commands:
```bash
just test                                                  # or cargo nextest run (all tests)
just workflow-test styles-legacy/apa.csl                   # or ./scripts/workflow-test.sh styles-legacy/apa.csl
just oracle styles-legacy/apa.csl                          # or node scripts/oracle.js styles-legacy/apa.csl
just check-core-quality                                    # or node scripts/report-core.js > /tmp/r.json && ...
```

Full catalogue and the **test-assertion rule** (no `contains()` with substrings <30 chars): [CODING_STANDARDS.md](docs/guides/CODING_STANDARDS.md). Test style (BDD `given/when/then`, `rstest` for parameterised): same doc.

## Optional: jj Change Stack

If `.jj` is present, see [JJ_AI_CHANGE_STACK.md](docs/guides/JJ_AI_CHANGE_STACK.md). Git remains the public surface. **jj skips all git hooks** — run commit-msg / pre-commit / pre-push manually before `jj git push`.

## Pointers

- Crate map: [crates/README.md](crates/README.md)
- Design principles: [docs/architecture/DESIGN_PRINCIPLES.md](docs/architecture/DESIGN_PRINCIPLES.md)
- Architecture index: [docs/architecture/README.md](docs/architecture/README.md)
- Live fidelity: `node scripts/report-core.js > /tmp/core-report.json`
- Compatibility dashboard: [docs/compat.html](docs/compat.html) (refresh with `node scripts/report-core.js --write-html`)
- Coding standards: [docs/guides/CODING_STANDARDS.md](docs/guides/CODING_STANDARDS.md)
- Locale authoring: [docs/guides/AUTHORING_LOCALES.md](docs/guides/AUTHORING_LOCALES.md)
- Domain Expert workflow: [docs/guides/DOMAIN_EXPERT.md](docs/guides/DOMAIN_EXPERT.md)
- Repo-local harness spec: [docs/specs/REPO_LOCAL_HARNESS.md](docs/specs/REPO_LOCAL_HARNESS.md)
- Frontmatter preflight: `./scripts/validate-frontmatter.sh --copilot-strict`
