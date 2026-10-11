// SPDX-License-Identifier: GPL-3.0-or-later
//! Runs the shared behaviour spec (spec/cases.json) against the Rust evaluator.

use expresso::calc::find_and_evaluate;

#[test]
fn shared_spec() {
    let spec: serde_json::Value =
        serde_json::from_str(include_str!("../../spec/cases.json")).expect("valid spec JSON");
    let cases = spec["cases"].as_array().expect("cases array");
    let mut failures = Vec::new();
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let want = case["result"].as_str();
        let got = find_and_evaluate(input).map(|e| e.display);
        if got.as_deref() != want {
            failures.push(format!("{input:?} -> {got:?} (want {want:?})"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} spec cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
