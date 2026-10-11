# Changelog

All notable changes to this project are documented here. Versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-11

### Added
- GNOME Shell 50 extension: inline results next to the caret in any
  text-input-v3 app (GTK3/4, Firefox, Qt6), hidden in password fields,
  dismissed by Enter and Escape.
- Evaluator: arithmetic, powers, factorial, modulo, calculator-style
  percentages, functions and constants, implicit multiplication, decimal and
  grouping commas, unclosed parentheses while typing.
- Shared behaviour spec (`spec/cases.json`) with 67 cases.
- Windows: Rust port of the evaluator passing the full spec, plus the bubble
  window and a `--demo` mode. Text-field reading is not implemented yet.
- Headless nested-GNOME-Shell end-to-end test harness.
