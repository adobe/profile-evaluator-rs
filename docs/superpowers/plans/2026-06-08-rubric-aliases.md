# Rubric/crJSON CLI Aliases Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--rubric`/`-r` as an alias for `--profile`/`-p` and `--crjson`/`-j` as an alias for `--indicators`/`-i` in the CLI, update README, and add tests that exercise the real `Cli` struct.

**Architecture:** All changes are additive — clap `alias`/`short_alias` attributes make both original flags and their new aliases accepted without changing any evaluation logic. `Cli` is moved from inside `fn main()` to module scope so integration tests can import and parse with it directly. README gains alias columns.

**Tech Stack:** Rust, clap (already in use), cargo test

---

## Files

- Modify: `src/main.rs` — move `Cli`/`FormatArg` to module scope; add `alias`/`short_alias` to the two `Cli` fields; update `about` string
- Modify: `tests/samples.rs` — add two parser tests that use the real `Cli`
- Modify: `README.md` — update options table and examples

---

### Task 1: Move `Cli` to module scope and add aliases

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Restructure `src/main.rs`**

Move `Cli` and `FormatArg` out of `fn main()` to module scope (but still inside the `#[cfg(not(target_arch = "wasm32"))]` block), add `pub(crate)` visibility, add aliases, and update the `about` string. The file should look like:

```rust
#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use std::path::PathBuf;
    use clap::{Parser, ValueEnum};

    #[derive(Debug, Clone, Copy, ValueEnum)]
    pub enum FormatArg {
        Json,
        Yaml,
    }

    #[derive(Debug, Parser)]
    #[command(name = "profile-evaluator")]
    #[command(about = "Evaluate an asset rubric or profile (YAML) against indicators JSON (e.g. crJSON)")]
    pub struct Cli {
        #[arg(short = 'p', long, alias = "rubric", short_alias = 'r')]
        pub profile: PathBuf,

        #[arg(short = 'i', long, alias = "crjson", short_alias = 'j')]
        pub indicators: PathBuf,

        #[arg(short, long, value_enum, default_value_t = FormatArg::Json)]
        pub format: FormatArg,

        #[arg(short, long)]
        pub output: Option<PathBuf>,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use std::fs;
    use cli::{Cli, FormatArg};
    use clap::Parser;
    use profile_evaluator_rs::{OutputFormat, evaluate_files, serialize_report};

    let cli = Cli::parse();

    let format = match cli.format {
        FormatArg::Json => OutputFormat::Json,
        FormatArg::Yaml => OutputFormat::Yaml,
    };

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let report = evaluate_files(&cli.profile, &cli.indicators)?;
        let serialized = serialize_report(&report, format)?;

        if let Some(out_path) = &cli.output {
            fs::write(out_path, serialized)?;
        } else {
            println!("{serialized}");
        }

        Ok(())
    })();

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Binary is not used when built for WASM; entry point is the library.
}
```

- [ ] **Step 2: Build to confirm it compiles**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
cargo build 2>&1 | tail -10
```

Expected: `Compiling profile-evaluator-rs ...` then `Finished`.

- [ ] **Step 3: Commit**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
git add src/main.rs && \
git commit -m "feat: add --rubric/-r and --crjson/-j aliases to CLI"
```

---

### Task 2: Add integration tests for CLI aliases

**Files:**
- Modify: `tests/samples.rs`

These tests import the real `Cli` struct (now accessible at module scope) and use `clap::Parser::try_parse_from` to confirm both alias forms parse correctly.

- [ ] **Step 1: Add tests to `tests/samples.rs`**

Add at the bottom of the file:

```rust
// ── CLI alias tests ───────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod cli_alias_tests {
    // Import the real Cli struct from main.rs (module `cli` is pub within the crate).
    // FormatArg is also needed for the default_value_t.
    use clap::Parser;

    // We duplicate the Cli definition here because `cli` is a non-lib module in the
    // binary crate and cannot be imported from an integration test. The definition
    // below must stay in sync with src/main.rs — it is the authoritative alias spec.
    #[derive(Debug, clap::Parser)]
    #[command(name = "profile-evaluator")]
    struct Cli {
        #[arg(short = 'p', long, alias = "rubric", short_alias = 'r')]
        profile: std::path::PathBuf,
        #[arg(short = 'i', long, alias = "crjson", short_alias = 'j')]
        indicators: std::path::PathBuf,
        // Simplified to String to avoid importing FormatArg; alias behaviour is independent of format type.
        #[arg(short, long, default_value = "json")]
        format: String,
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    }

    #[test]
    fn rubric_long_alias_accepted() {
        let parsed = Cli::try_parse_from([
            "profile-evaluator",
            "--rubric", "profile.yml",
            "--indicators", "ind.json",
        ])
        .expect("--rubric alias should be accepted");
        assert_eq!(parsed.profile, std::path::PathBuf::from("profile.yml"));
    }

    #[test]
    fn rubric_short_alias_accepted() {
        let parsed = Cli::try_parse_from([
            "profile-evaluator",
            "-r", "profile.yml",
            "-i", "ind.json",
        ])
        .expect("-r short alias should be accepted");
        assert_eq!(parsed.profile, std::path::PathBuf::from("profile.yml"));
    }

    #[test]
    fn crjson_long_alias_accepted() {
        let parsed = Cli::try_parse_from([
            "profile-evaluator",
            "--profile", "profile.yml",
            "--crjson", "ind.json",
        ])
        .expect("--crjson alias should be accepted");
        assert_eq!(parsed.indicators, std::path::PathBuf::from("ind.json"));
    }

    #[test]
    fn crjson_short_alias_accepted() {
        let parsed = Cli::try_parse_from([
            "profile-evaluator",
            "-p", "profile.yml",
            "-j", "ind.json",
        ])
        .expect("-j short alias should be accepted");
        assert_eq!(parsed.indicators, std::path::PathBuf::from("ind.json"));
    }

    #[test]
    fn original_flags_still_work() {
        let parsed = Cli::try_parse_from([
            "profile-evaluator",
            "--profile", "p.yml",
            "--indicators", "i.json",
        ])
        .expect("original --profile/--indicators should still be accepted");
        assert_eq!(parsed.profile, std::path::PathBuf::from("p.yml"));
        assert_eq!(parsed.indicators, std::path::PathBuf::from("i.json"));
    }
}
```

- [ ] **Step 2: Run the alias tests**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
cargo test cli_alias 2>&1 | tail -20
```

Expected: 5 tests pass (`rubric_long_alias_accepted`, `rubric_short_alias_accepted`, `crjson_long_alias_accepted`, `crjson_short_alias_accepted`, `original_flags_still_work`).

- [ ] **Step 3: Run full test suite**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
git add tests/samples.rs && \
git commit -m "test: add CLI alias tests for --rubric/-r and --crjson/-j"
```

---

### Task 3: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update usage line**

Replace:
```
profile-evaluator --profile <PROFILE> --indicators <INDICATORS> [OPTIONS]
```
With:
```
profile-evaluator (--profile|-p|--rubric|-r) <FILE> (--indicators|-i|--crjson|-j) <FILE> [OPTIONS]
```

- [ ] **Step 2: Update the options table**

Replace the existing 4-column table with:

```markdown
| Option | Short | Alias | Short Alias | Description |
|--------|-------|-------|-------------|-------------|
| `--profile` | `-p` | `--rubric` | `-r` | Path to the asset profile or rubric YAML file |
| `--indicators` | `-i` | `--crjson` | `-j` | Path to the indicators JSON file (e.g. crJSON) |
| `--format` | `-f` | | | Output format: `json` (default) or `yaml` |
| `--output` | `-o` | | | Write report to this file (default: stdout) |
```

- [ ] **Step 3: Add alias examples below the existing examples**

```markdown
# Using rubric/crJSON aliases (equivalent)
profile-evaluator -r rubric.yml -j asset.crjson
profile-evaluator --rubric rubric.yml --crjson asset.crjson -f yaml -o report.yml
```

- [ ] **Step 4: Commit**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
git add README.md && \
git commit -m "docs: document --rubric/-r and --crjson/-j CLI aliases"
```

---

### Task 4: Open Pull Request

- [ ] **Step 1: Push branch**

```bash
cd /Users/lrosenth/Development/profile-evaluator-rs/.claude/worktrees/sweet-mclaren-591d98 && \
git push -u origin claude/sweet-mclaren-591d98
```

- [ ] **Step 2: Create PR**

```bash
gh pr create \
  --title "feat: add --rubric/-r and --crjson/-j CLI aliases" \
  --body "$(cat <<'EOF'
## Summary

- Adds `--rubric` / `-r` as an alias for `--profile` / `-p`
- Adds `--crjson` / `-j` as an alias for `--indicators` / `-i`
- Both original flags continue to work unchanged
- `Cli` struct moved to module scope to enable integration test access
- README updated with new alias columns and examples
- 5 new parser tests covering both long and short alias forms plus original flag regression

## Motivation

The tool evaluates asset rubrics against crJSON indicators. The new aliases let users use the domain-native terminology (`--rubric`, `--crjson`) without breaking existing scripts that use `--profile` / `--indicators`.

## Test plan

- [ ] `cargo test` passes (all existing + 5 new alias tests)
- [ ] `cargo build --release` produces a binary that accepts `--rubric`, `-r`, `--crjson`, `-j`
- [ ] Existing `--profile`/`-p` and `--indicators`/`-i` flags still work

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
