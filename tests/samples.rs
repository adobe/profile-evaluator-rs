/*
Copyright 2026 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;

use profile_evaluator_rs::{OutputFormat, evaluate_files, evaluate_texts, serialize_report};
use serde_json::Value;

fn run_case(name: &str) {
    let profile = format!("testfiles/{}_profile.yml", name);
    let indicators = format!("testfiles/{}_indicators.json", name);

    let report = evaluate_files(&profile, &indicators).expect("evaluation should succeed");

    let expected_json_path = format!("output/{}_indicators_report.json", name);
    let expected_json: Value =
        serde_json::from_str(&fs::read_to_string(&expected_json_path).expect("expected json file"))
            .expect("valid expected json");
    assert_eq!(report, expected_json, "json mismatch for case {name}");

    let actual_yaml = serialize_report(&report, OutputFormat::Yaml).expect("yaml serialization");
    let actual_yaml_value: Value = serde_yaml::from_str(&actual_yaml).expect("actual yaml parse");

    let expected_yaml_path = format!("output/{}_indicators_report.yml", name);
    let expected_yaml_text = fs::read_to_string(&expected_yaml_path).expect("expected yaml file");
    let expected_yaml_value: Value =
        serde_yaml::from_str(&expected_yaml_text).expect("expected yaml parse");

    assert_eq!(
        actual_yaml_value, expected_yaml_value,
        "yaml mismatch for case {name}"
    );
}

#[test]
fn validates_all_sample_profiles() {
    let cases = [
        "blocks",
        "camera",
        "expression",
        "genai",
        "includes",
        "no_manifests",
        "signature",
    ];

    for case in cases {
        run_case(case);
    }
}

#[test]
fn cli_roundtrip_json_matches_expected_case() {
    let report = evaluate_files(
        "testfiles/no_manifests_profile.yml",
        "testfiles/no_manifests_indicators.json",
    )
    .expect("evaluation should succeed");
    let rendered = serialize_report(&report, OutputFormat::Json).expect("json serialization");
    let rendered_value: Value = serde_json::from_str(&rendered).expect("valid json");

    let expected_path = Path::new("output/no_manifests_indicators_report.json");
    let expected: Value =
        serde_json::from_str(&fs::read_to_string(expected_path).expect("read expected"))
            .expect("parse expected json");

    assert_eq!(rendered_value, expected);
}

// ── Error reporting tests ─────────────────────────────────────────────────────

/// Returns the first statement entry from the report's `statements` array.
fn first_statement(report: &Value) -> &Value {
    report["statements"][0][0]
        .as_object()
        .map(|_| &report["statements"][0][0])
        .expect("expected a statement entry")
}

#[test]
fn expression_error_adds_error_entry() {
    // A statement whose expression calls a function that does not exist.
    let profile = r#"
---
profile_metadata:
  name: Error Test
  language: en
---
- id: bad_expr
  expression: nonexistent_func(@)
  report_text: should not appear
"#;
    let indicators = "{}";

    let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
    let stmt = first_statement(&report);

    // Must have an "error" entry
    assert!(
        stmt.get("error").is_some(),
        "expected 'error' key in statement, got: {stmt}"
    );

    // Error must carry both "kind" and "message" from the library
    let error = &stmt["error"];
    assert!(
        error.get("kind").and_then(Value::as_str).is_some(),
        "expected 'kind' in error object, got: {error}"
    );
    assert!(
        error.get("message").and_then(Value::as_str).is_some(),
        "expected 'message' in error object, got: {error}"
    );

    // Must NOT have a "value" entry (value is meaningless when eval failed)
    assert!(
        stmt.get("value").is_none(),
        "expected no 'value' key when expression errored, got: {stmt}"
    );

    // Must NOT have "report_text" when expression errored
    assert!(
        stmt.get("report_text").is_none(),
        "expected no 'report_text' when expression errored, got: {stmt}"
    );
}

#[test]
fn expression_error_kind_is_function_error() {
    // Calling an undefined function triggers a FunctionError in json-formula.
    let profile = r#"
---
profile_metadata:
  name: Kind Test
  language: en
---
- id: kind_stmt
  expression: undefined_func()
  report_text: ignored
"#;
    let indicators = "{}";

    let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
    let stmt = first_statement(&report);
    let kind = stmt["error"]["kind"].as_str().expect("kind must be a string");

    assert_eq!(
        kind, "FunctionError",
        "expected FunctionError for unknown function, got: {kind}"
    );
}

#[test]
fn expression_error_message_contains_function_name() {
    let profile = r#"
---
profile_metadata:
  name: Message Test
  language: en
---
- id: msg_stmt
  expression: my_missing_func(@)
  report_text: ignored
"#;
    let indicators = "{}";

    let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
    let stmt = first_statement(&report);
    let message = stmt["error"]["message"].as_str().expect("message must be a string");

    assert!(
        message.contains("my_missing_func"),
        "expected function name in error message, got: {message}"
    );
}

#[test]
fn successful_expression_with_missing_field_has_debug_entry() {
    // Accessing a field that doesn't exist returns null (no error) but the
    // library emits diagnostic messages explaining what went wrong.
    let profile = r#"
---
profile_metadata:
  name: Debug Test
  language: en
---
- id: debug_stmt
  expression: missing_field
  report_text:
    "null":
      en: field not found
"#;
    let indicators = "{}";

    let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
    let stmt = first_statement(&report);

    // Must have succeeded (value present, no error)
    assert!(
        stmt.get("error").is_none(),
        "expected no 'error' for a successful evaluation, got: {stmt}"
    );
    assert_eq!(
        stmt["value"],
        serde_json::Value::Null,
        "expected null value for missing field"
    );

    // Must have a non-empty debug array
    let debug = stmt.get("debug").expect("expected 'debug' key when library emits diagnostics");
    let debug_arr = debug.as_array().expect("'debug' must be an array");
    assert!(
        !debug_arr.is_empty(),
        "expected at least one debug message, got empty array"
    );

    // At least one message should mention the missing field name
    let mentions_field = debug_arr
        .iter()
        .filter_map(serde_json::Value::as_str)
        .any(|s| s.contains("missing_field"));
    assert!(
        mentions_field,
        "expected a debug message mentioning 'missing_field', got: {debug_arr:?}"
    );
}

#[test]
fn valid_expression_has_no_error_entry() {
    // A well-formed expression must produce a value and no error entry.
    let profile = r#"
---
profile_metadata:
  name: No Error Test
  language: en
---
- id: ok_stmt
  expression: length(@)
  report_text:
    "0":
      en: empty
    "1":
      en: one item
"#;
    let indicators = r#"[42]"#;

    let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
    let stmt = first_statement(&report);

    assert!(
        stmt.get("error").is_none(),
        "expected no 'error' key for valid expression, got: {stmt}"
    );
    assert!(
        stmt.get("value").is_some(),
        "expected 'value' for valid expression, got: {stmt}"
    );
}

// ── CLI alias tests ───────────────────────────────────────────────────────────

// These tests mirror the `Cli` struct from src/main.rs.  Integration tests
// cannot import from a binary crate's modules, so we duplicate the struct here.
// The `format` field uses `String` instead of `FormatArg` (a `ValueEnum`) for
// simplicity — the alias behaviour being tested is independent of that field's type.
// Keep this struct's `profile` and `indicators` attributes in sync with src/main.rs.
mod cli_alias_tests {
    use clap::Parser;

    #[derive(Debug, Parser)]
    #[command(name = "profile-evaluator")]
    struct Cli {
        #[arg(short = 'p', long, alias = "rubric", short_alias = 'r')]
        profile: std::path::PathBuf,
        #[arg(short = 'i', long, alias = "crjson", short_alias = 'j')]
        indicators: std::path::PathBuf,
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

mod warn_if_matched_tests {
    use profile_evaluator_rs::evaluate_texts;
    use serde_json::{Value, json};

    const INDICATORS: &str = r#"{"bad": ["c2pa.edited", "c2pa.opened"], "good": []}"#;

    fn statements(report: &Value) -> &Vec<Value> {
        report["statements"][0]
            .as_array()
            .expect("expected a statements section")
    }

    #[test]
    fn warn_match_value_is_the_match_list() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    warn:
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!(["c2pa.edited", "c2pa.opened"]));
        assert_eq!(
            stmt["report_text"],
            json!(r#"discouraged: ["c2pa.edited", "c2pa.opened"]"#),
            "matches must render as a JSON-style array, not handlebars' unquoted form"
        );
    }

    #[test]
    fn warn_match_falls_back_to_false_text_without_warn_key() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    'false':
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!(["c2pa.edited", "c2pa.opened"]));
        assert_eq!(
            stmt["report_text"],
            json!(r#"discouraged: ["c2pa.edited", "c2pa.opened"]"#)
        );
    }

    #[test]
    fn warn_without_match_reports_empty_list_and_true_text() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'good[]'
  reportText:
    'true':
      en: 'clean (matches={{matches}})'
    warn:
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!([]));
        assert_eq!(
            stmt["report_text"],
            json!("clean (matches=[])"),
            "an empty match list must interpolate as [], not as a missing-key marker"
        );
    }

    #[test]
    fn warn_with_null_result_reports_empty_list() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: missing_field
  reportText:
    'true':
      en: clean
    warn:
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!([]));
        assert_eq!(stmt["report_text"], json!("clean"));
    }

    #[test]
    fn fail_match_keeps_boolean_value_and_interpolates_matches() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: failed
  failIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    'false':
      en: 'failing: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!(false));
        assert_eq!(
            stmt["report_text"],
            json!(r#"failing: ["c2pa.edited", "c2pa.opened"]"#)
        );
    }

    #[test]
    fn fail_without_match_keeps_true_value() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: failed
  failIfMatched: true
  expression: 'good[]'
  reportText:
    'true':
      en: clean
    'false':
      en: 'failing: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!(true));
        assert_eq!(stmt["report_text"], json!("clean"));
    }

    #[test]
    fn matches_items_supports_each_blocks() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{#each matches_items}}[{{this}}]{{/each}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["report_text"], json!("[c2pa.edited][c2pa.opened]"));
    }

    #[test]
    fn matches_does_not_leak_into_later_statements() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    warn:
      en: 'discouraged: {{matches}}'
- id: later
  report_text: 'leaked? {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let later = &statements(&report)[1];

        let text = later["report_text"].as_str().expect("report_text string");
        assert!(
            text.contains("🔴 Missing: matches()"),
            "matches must be removed after the owning statement, got: {text}"
        );
    }

    #[test]
    fn match_list_is_visible_to_dependent_expressions() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    warn:
      en: warned
- id: dependent
  expression: 'length(@.profile.warned)'
  reportText:
    '0':
      en: no warnings
    '2':
      en: two warnings
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let dependent = &statements(&report)[1];

        assert_eq!(
            dependent["value"],
            json!(2),
            "profile.<id> must carry the same match list the report shows"
        );
        assert_eq!(dependent["report_text"], json!("two warnings"));
    }

    #[test]
    fn default_report_text_key_is_used_when_status_key_absent() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    default:
      en: 'fallback: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(
            stmt["report_text"],
            json!(r#"fallback: ["c2pa.edited", "c2pa.opened"]"#)
        );
    }

    #[test]
    fn bare_field_expression_matches_like_a_projection() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'bad'
  reportText:
    'true':
      en: clean
    warn:
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!(["c2pa.edited", "c2pa.opened"]));
        assert_eq!(
            stmt["report_text"],
            json!(r#"discouraged: ["c2pa.edited", "c2pa.opened"]"#)
        );
    }

    #[test]
    fn bare_field_and_flatten_agree_for_flat_arrays() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: bare
  warnIfMatched: true
  expression: 'bad'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
- id: projected
  warnIfMatched: true
  expression: 'bad[]'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let items = statements(&report);

        assert_eq!(items[0]["value"], items[1]["value"]);
        assert_eq!(items[0]["report_text"], items[1]["report_text"]);
    }

    #[test]
    fn bare_field_and_flatten_diverge_on_nested_and_scalar() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: nested_flattened
  warnIfMatched: true
  expression: 'nested[]'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
- id: nested_bare
  warnIfMatched: true
  expression: 'nested'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
- id: scalar_flattened
  warnIfMatched: true
  expression: 'scalar[]'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
- id: scalar_bare
  warnIfMatched: true
  expression: 'scalar'
  reportText:
    'true':
      en: clean
    warn:
      en: '{{matches}}'
"#;
        let indicators = r#"{"nested": [[0, 1], [1, 2], 3], "scalar": 42}"#;
        let report = evaluate_texts(profile, indicators, None).expect("evaluation must succeed");
        let items = statements(&report);

        assert_eq!(
            items[0]["value"],
            json!([0, 1, 1, 2, 3]),
            "flatten collapses one level"
        );
        assert_eq!(
            items[1]["value"],
            json!([[0, 1], [1, 2], 3]),
            "a bare field is returned verbatim"
        );
        assert_eq!(
            items[2]["value"],
            json!([]),
            "flatten on a scalar yields null, so nothing matched"
        );
        assert_eq!(
            items[3]["value"],
            json!([42]),
            "a bare scalar is wrapped into a one-item match list"
        );
    }

    #[test]
    fn empty_array_field_warns_clean() {
        let profile = r#"
---
profile_metadata:
  language: en
---
- id: warned
  warnIfMatched: true
  expression: 'good'
  reportText:
    'true':
      en: clean
    warn:
      en: 'discouraged: {{matches}}'
"#;
        let report = evaluate_texts(profile, INDICATORS, None).expect("evaluation must succeed");
        let stmt = &statements(&report)[0];

        assert_eq!(stmt["value"], json!([]));
        assert_eq!(stmt["report_text"], json!("clean"));
    }
}
