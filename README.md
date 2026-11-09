# profile-evaluator-rs

Evaluate an asset rubric or trust profile (YAML) against trust indicators such as crJSON. Produces a report by running the provided expressions and template text over the indicator data.

## Building

Requires Rust (e.g. via [rustup](https://rustup.rs/)).

This crate depends on a local `json-formula-rs` at `../json-formula-rs`. Clone or symlink that repo as a sibling of this one, then:

```bash
cargo build --release
```

## WASM/Web App

The same `ui/` app used by the Tauri shell can also run in a browser against a WASM build of the evaluator library.

Build the browser bundle with:

```bash
./scripts/build-wasm.sh
```

That generates `ui/pkg/` using `wasm-pack` and the `wasm32-unknown-unknown` target.

To run the web app, serve the `ui/` directory over HTTP and open `ui/index.html`. For example:

```bash
python3 -m http.server 8000 -d ui
```

Then open `http://127.0.0.1:8000`.

Notes:
- browser mode uses the same HTML/CSS/JS UI as Tauri
- browser mode evaluates profiles fully in WASM
- YAML `include:` paths are still supported in native/Tauri builds, but not in the browser build because the browser cannot read sibling files from a local path

## Usage

```bash
profile-evaluator --profile <PROFILE> --indicators <INDICATORS> [OPTIONS]
# or using rubric/crJSON aliases:
profile-evaluator --rubric <RUBRIC> --crjson <CRJSON> [OPTIONS]
```

| Option | Short | Alias | Short Alias | Description |
|--------|-------|-------|-------------|-------------|
| `--profile` | `-p` | `--rubric` | `-r` | Path to the asset profile or rubric YAML file |
| `--indicators` | `-i` | `--crjson` | `-j` | Path to the indicators JSON file (e.g. crJSON) |
| `--format` | `-f` | | | Output format: `json` (default) or `yaml` |
| `--output` | `-o` | | | Write report to this file (default: stdout) |

### Examples

```bash
# JSON report to stdout
profile-evaluator -p profile.yml -i indicators.json

# YAML report to a file
profile-evaluator -p profile.yml -i indicators.json -f yaml -o report.yml

# Using rubric/crJSON aliases (equivalent)
profile-evaluator -r rubric.yml -j asset.crjson
profile-evaluator --rubric rubric.yml --crjson asset.crjson -f yaml -o report.yml
```

## Match-list statements

A statement whose expression returns the list of offending items can be declared with one of two flags instead
of returning a boolean. `null` and an empty array both mean "nothing matched".

| Flag | Nothing matched | Matched |
|---|---|---|
| `failIfMatched: true` | `value: true` | `value: false` |
| `warnIfMatched: true` | `value: []` | `value: [<matched items>]` |

Report text is selected by the match state, not by the reported value: `true` when nothing matched, `false` for
a `failIfMatched` match, `warn` for a `warnIfMatched` match. A `warn` state falls back to the `false` text when
no `warn` key exists, and any state falls back to a `default` key.

Inside a match-list statement's report text:

- `{{matches}}` — the matched items as a JSON-style array, e.g. `["c2pa.edited", "c2pa.opened"]`; `[]` when
  nothing matched
- `{{matches_items}}` — the raw list, for `{{#each matches_items}}` blocks

Both are scoped to the statement they belong to. Because a `warnIfMatched` statement's value is a list, a
dependent expression reading `profile.<id>` sees an empty (falsy) list when there is nothing to warn about —
use `length(@.profile.<id>)` rather than bare truthiness.

```yaml
- id: legacy_action_parameters
  warnIfMatched: true
  expression: |
    manifests[0].assertions.'c2pa.actions.v2'.actions[?parameters.legacyField != null].action
  reportText:
    'true':
      en: No actions use the deprecated parameter field
    warn:
      en: 'Actions using the deprecated parameter field: {{matches}}'
```

## Development

Run tests (expects `testfiles/` and `output/` with sample profiles and expected reports):

```bash
cargo test
```

### WASM tests

The library includes tests that run in the WASM build to verify evaluation and the JS-facing export. Run them with [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/):

```bash
wasm-pack test --node
```

This compiles the crate for `wasm32-unknown-unknown` and runs the WASM tests in Node. To run only the library’s unit tests (and skip the binary/integration tests), use:

```bash
wasm-pack test --node -- --lib
```

## GUI (Tauri)

This repository now includes a Tauri desktop app at `src-tauri/` with:
- editable source JSON field and `Select File...` loader
- editable YAML profile field and `Select File...` loader
- `Evaluate Profile` button that runs the Rust evaluator library
- scrollable JSON result panel with syntax highlighting and collapsible hierarchy

Run it with:

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

## License

See repository for license information.
