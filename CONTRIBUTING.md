# Contributing

## The one rule: spec first

The evaluator exists twice, `linux/extension/expresso@bulrok/calc.js` and
`windows/src/calc.rs`, and both must behave identically. The contract is
`spec/cases.json`.

1. Add or change the case in `spec/cases.json`.
2. Make `make test-linux` pass (JS).
3. Make `make test-windows` pass (Rust). This runs on Linux too.

A change that touches only one evaluator without a spec case won't be merged.

## Checks

```bash
make test          # both evaluators against the spec, plus unit tests
make lint          # cargo fmt --check, clippy -D warnings
make test-nested   # Linux end-to-end, if you have GNOME 50
```

CI runs the same on Ubuntu (GJS) and Windows (Rust).

## Style

- **JS:** GNOME Shell conventions (ES modules, 4-space indent,
  `connectObject`/`disconnectObject`). The extension must leave nothing
  behind on `disable()`: lock and unlock cycle it.
- **Rust:** `cargo fmt`; keep `unsafe` confined to `src/win/`.
- **Comments:** explain *why*, especially workarounds for toolkit quirks.

## Releasing

1. Bump `version` in `windows/Cargo.toml` and `version-name` (plus the
   integer `version`) in `metadata.json`. `make check-version` verifies they
   agree.
2. Move the `Unreleased` notes in `CHANGELOG.md` under the new version.
3. Tag `vX.Y.Z` and push the tag. The release workflow builds the
   extension zip and the Windows exe and publishes them.
