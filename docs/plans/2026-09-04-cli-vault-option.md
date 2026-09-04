# Implementation plan: CLI vault path option

This plan implements the behavior described in [the CLI vault path specification](../specs/2026-09-04-cli-vault-option.md).

## 1. Introduce Clap-based CLI parsing

Add the `clap` crate with its derive feature and update `src/main.rs` so a derived `Parser` returns structured invocation options instead of only a `bool` for slim mode. The result should carry:

- `slim: bool`
- `vault_path: Option<PathBuf>`

Define the explicit vault path as a typed `PathBuf` argument with a value parser that expands `~` using the same `expand_tilde` helper used by configuration. Add an optional positional `VAULT` argument with a parser that requires an existing regular `.kdbx` file. Let Clap reject missing values, repeated options, unknown options, and additional positional arguments. Reject supplying both `--vault` and positional `VAULT`, and ensure parsing completes before `ratatui::init()`.

Explicitly preserve the existing parameters and aliases in the Clap definition: `--slim`, `--version`/`-V`/`-v`, and `--help`/`-H`/`-h`. Disable Clap's automatic help/version flags only if needed to define these exact legacy aliases, and retain the current version/description output semantics.

Avoid placing passwords or keyfile contents in the parsed options.

## 2. Make the selected path explicit in application startup

Adjust startup in `src/main.rs` and `src/app.rs` so the parsed `vault_path`, when present, overrides `Config::default_database` in memory before the `App` is created. Keep the existing fallback behavior when no override is provided.

The selected path must remain in `app.config.default_database`, because the existing login and mutation code uses that field for unlock, save, import, export, delete, and password-change operations. Do not call `save_config` merely because a CLI override was supplied.

If the app currently saves config as part of unrelated settings changes, add an explicit transient/override distinction or otherwise ensure the command-line path is not persisted. Verify this behavior against settings editing and the first-run path.

## 3. Update help and error behavior

Expand the help text to show the option syntax, purpose, and precedence. Add concise parser errors for:

- `--vault` without a value;
- `--vault --slim` or another option used as its value;
- a second `--vault`;
- unknown options and positional arguments.

Use a non-zero process exit status for invalid CLI input. Keep `--version` and `--help` usable independently and preserve their existing aliases.

## 4. Add focused tests

Add unit tests around the Clap parser in `src/main.rs` or a small testable CLI parsing module. Cover:

- no options;
- `--slim`;
- `--vault /tmp/work.kdbx`;
- an existing positional `/tmp/work.kdbx`;
- `--vault` with a path containing spaces;
- `--vault ~/work.kdbx` expanding consistently;
- combined `--vault /tmp/work.kdbx --slim`;
- existing `--slim` behavior;
- `--version`, `-V`, and `-v` aliases;
- `--help`, `-H`, and `-h` aliases;
- missing vault value;
- an option mistakenly used as the value;
- duplicate vault options;
- unknown options and positional arguments.
- both `--vault /tmp/a.kdbx /tmp/b.kdbx` forms together.

Add an application/startup test proving that a CLI path overrides a configured `default_database` in memory and that the config value is not rewritten. Reuse the existing database tests where possible rather than duplicating unlock behavior.

## 5. Validate end-to-end behavior

Run formatting, compilation, and the complete test suite:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Manually verify the following with temporary vaults:

1. Open a password-only vault through `--vault`.
2. Open a vault that uses the configured keyfile.
3. Modify an entry and confirm the selected vault changes.
4. Run with a CLI vault while a different `default_database` is configured, then confirm the config is unchanged.
5. Open an existing vault using the positional form.
6. Use an invalid invocation and confirm no terminal UI is initialized.

## 6. Documentation follow-up

Add examples to `readme.md` under “Getting started” showing both `jaiba --vault ~/Documents/work.kdbx` and `jaiba ~/Documents/work.kdbx`, and state that the command-line forms override `default_database` for the current session only. Keep the config example focused on persistent defaults.

## Likely files

- `src/main.rs` — parser, option model, help, and parser tests.
- `src/app.rs` — startup wiring and transient override handling, if required.
- `readme.md` — user-facing usage example.
- `docs/specs/2026-09-04-cli-vault-option.md` — behavior contract.
- `docs/plans/2026-09-04-cli-vault-option.md` — this implementation plan.

## Risks and mitigations

- **Accidental config persistence:** keep the override in memory and test the config file before and after a CLI launch.
- **Incorrect path precedence:** resolve the override before app initialization and add a test with distinct configured and CLI paths.
- **Parser regressions:** test all existing aliases and reject malformed input before terminal initialization.
- **Keyfile confusion:** document and test that `--vault` changes only the vault path; the configured keyfile and interactive master-password flow remain unchanged.
