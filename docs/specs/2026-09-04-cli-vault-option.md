# CLI vault path option

## Summary

Add a command-line option that lets a user choose the KeePass vault file to open for one Jaiba invocation, without editing `jaiba_config.toml`. Use the `clap` crate as the single source of truth for parsing, validation, and generated help.

The option is:

```text
jaiba --vault <PATH>
```

For an existing `.kdbx` file, the path may also be supplied positionally:

```text
jaiba <VAULT>
```

`--vault` is an invocation-level override for the configured `default_database`. It does not change the saved configuration. The existing interactive master-password prompt and optional configured keyfile continue to protect the vault.

## Motivation

Today, opening a particular `.kdbx` file requires changing `default_database` in the configuration file. That is inconvenient for users who regularly work with more than one vault, launch Jaiba from scripts, or want to inspect a vault without changing their normal default.

## Requirements

### Functional requirements

1. Jaiba accepts `--vault <PATH>` and uses the supplied path as the active database path.
2. Jaiba accepts a positional `<VAULT>` path when it refers to an existing regular `.kdbx` file.
3. A positional path and `--vault` cannot be supplied together.
4. The command-line path takes precedence over `default_database` from `~/.config/rama/jaiba_config.toml`.
5. The path is used for the complete session, including unlock, create-if-missing behavior, save, delete, import, export, and password-change operations.
6. The command-line path is not written back to the configuration file when the process exits.
7. Paths containing spaces and paths using `~` are accepted. Tilde expansion must match configuration behavior.
8. A missing or unreadable vault produces the existing actionable error or missing-database flow, with the selected path included in the message.
9. The vault's master password is still requested interactively. The configured `keyfile`, when present, is still used; `--vault` does not accept or expose a password or keyfile secret.
10. `--help` documents both forms, their value requirements, precedence, and examples.
11. Existing options (`--slim`, `--version`, and help aliases) remain compatible.

### CLI contract

```text
jaiba [OPTIONS] [VAULT]

Arguments:
    [VAULT]         Open an existing .kdbx vault for this session

Options:
    --vault <PATH>  Open PATH as the KeePass vault for this session
    --slim          Use the slim interface
    -v, -V, --version
                    Print version information
    -h, -H, --help  Print help information
```

The option requires exactly one non-empty value. These invocations are errors:

```text
jaiba --vault
jaiba --vault --slim
```

Supplying both vault forms is also an error:

```text
jaiba --vault passwords.kdbx other.kdbx
```

An unknown option, a second `--vault`, or an unexpected positional argument should fail before the terminal UI is initialized and return a non-zero exit status. Error output should identify the problem and show the help hint where useful.

The implementation must preserve all existing CLI parameters and aliases while migrating to Clap:

- `--slim` remains available.
- `--version`, `-V`, and `-v` continue to print version information.
- `--help`, `-H`, and `-h` continue to print help information.

Clap's derive parser should own these flags and their errors; no parallel hand-written option parser should remain.

### Precedence

The effective database path is resolved in this order:

1. `--vault <PATH>` from the command line.
2. Positional `<VAULT>` from the command line.
3. `default_database` from the loaded configuration.
4. Existing first-run/default-database behavior when neither is supplied.

Only the effective path for the current process changes. Other configuration values, including `keyfile`, remain sourced from the configuration file.

### Compatibility and security

- Existing invocations without `--vault` behave exactly as they do today.
- Existing invocations with `--slim`, version flags, and help flags retain their behavior.
- The option accepts a filesystem path only; it must not accept a password, keyfile contents, or any secret-bearing value.
- The path may appear in error messages because it is user-supplied filesystem metadata, but passwords and key material must never be logged or included in help text.
- The implementation must not load the vault before the normal unlock flow or bypass the master-password prompt.

## Acceptance criteria

- `jaiba --vault ~/Documents/work.kdbx` starts with `~/Documents/work.kdbx` as the active vault.
- When both `--vault ~/work.kdbx` and `default_database = "~/personal.kdbx"` exist, the work vault is selected.
- Starting without `--vault` still selects `default_database` exactly as before.
- After using `--vault`, changing or saving data targets the selected vault, not the configured default.
- Exiting after a `--vault` invocation leaves the config's `default_database` unchanged.
- `--help` shows the option and invalid/missing values are rejected without initializing Ratatui.
- An existing positional `.kdbx` path selects that vault, while a missing or non-`.kdbx` positional path is rejected.
- Existing version, help, and slim-mode behavior remains covered by tests.

## Out of scope

- Changing the config-file format.
- Adding a command-line password or keyfile option.
- Selecting a vault interactively after startup.
- Opening multiple vaults in one process.
- Supporting vault formats other than `.kdbx` for the positional form.
