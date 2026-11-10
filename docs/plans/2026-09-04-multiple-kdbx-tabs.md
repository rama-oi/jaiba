# Implementation Plan: Open Multiple KDBX Files in Tabs

## Outcome

Replace the single-vault state in `App` with a collection of per-vault tab
sessions plus a selected-tab index. Render the selected sessions through
Ratatui's `Tabs` widget and add an explicit open-vault prompt flow. Preserve
the current login path for `default_database`, while making all unlocked-vault
operations resolve through the selected session.

## Design

### State model

Introduce a `VaultTab` (name to be finalized) containing at least:

- canonical database path and display label;
- `Database` and its `DatabaseKey`;
- decrypted `Vec<Entry>` and filtered indices;
- query and `TableState`;
- edit entry/original/target and edit UI state;
- per-vault import/export and password-change state, or a documented rule that
  these modal flows block tab switching until completed;
- dirty/close-confirmation state if existing save behavior cannot fully infer it.

`App` should retain `Vec<VaultTab>` and `active_tab`, while keeping global
theme, clipboard, auto-lock, terminal mode, and application lifecycle state at
the top level. Add accessors such as `active_tab()` and `active_tab_mut()` so
input and UI code does not index the vector inconsistently.

The safest first implementation is to move all currently vault-bound fields
together, rather than leaving aliases such as `app.entries` alongside the new
collection. This prevents saving one tab with another tab's key or selection.

### Open flow

Add an explicit modal state, likely `OpeningDatabase`, with path and password
buffers and an optional keyfile-path buffer. Keep it separate from the initial
login state because the initial login uses configuration and must not mutate
the current tab collection.

Extract the common unlock operation around `db::unlock_database`, returning a
fully initialized `VaultTab` only after the database and entries are both
available. Normalize/validate the path before unlocking, check for an existing
tab, and append/select only on success. Clear password buffers on success,
cancel, and error.

Decide and document the keyfile UX during implementation: the recommended
option is to let the open flow accept an optional per-tab keyfile path, with an
empty value meaning password-only. Do not reuse `config.keyfile` implicitly
for a path selected by the user.

### Input routing

Add open/switch/close commands to `input/command.rs` or a focused tab command
module. Reserve a non-conflicting set of shortcuts, for example an unmodified
open key and Ctrl-based tab navigation/close, after checking the current
command table and terminal behavior. Update `ui/index.rs` help output and the
corresponding help/modal text in other screens.

Centralize “active tab required” checks. Existing index, edit, settings,
clipboard, import, export, and password-change handlers should obtain the
active session through the accessor instead of reading global vault fields.
When a modal operation is active, either keep tab switching disabled until it
finishes or move its buffers into the selected `VaultTab`; do not allow a
switch to retarget an in-progress save/import/password change.

### Rendering

Create a shared tab-strip renderer, likely in `ui/mod.rs` or a new `ui/tabs.rs`:

1. Build `Tabs` titles from the tab labels.
2. Use `active_tab` as the selected index.
3. Apply the existing theme colors for normal, selected, and highlight states.
4. Render it above the screen-specific content with a stable height.

Refactor index layout constraints to reserve the tab-strip row. Decide whether
settings/edit use the same shared layout helper so the active vault remains
visible across all unlocked screens. Ensure narrow terminals do not panic and
that long/duplicate labels are shortened consistently.

Reserve two rows only when at least two tabs are open; otherwise render no tab
strip and do not leave an empty gap. Enable Crossterm mouse capture for the
application lifecycle. Handle left-button mouse events and map their column to
the same label/divider ranges used by the `Tabs` renderer, selecting the
clicked tab while ignoring clicks outside the titles.

### Locking and lifecycle

Update `maybe_auto_lock` to drain or clear every `VaultTab`, including database
keys and edit/password buffers, then return to `Screen::Login`. Reset the tab
collection and active index in one helper to avoid partial cleanup. Verify that
closing a tab drops its `Database` and key immediately and that a failed open
does not alter the existing collection.

Review the initial create/unlock paths in `input/login.rs` so they construct a
tab and select it rather than populating singleton fields. Keep writing
`config.default_database` only for the existing create/default-login behavior.

When `default_database` is unset at startup, enter the open-database dialog
immediately. Canceling with no tabs returns to login so the user can choose the
create-database flow. A configured-but-missing default path keeps the existing
missing-database behavior.

The open path field performs dependency-free completion with `read_dir`: it
completes shared prefixes, cycles repeated-Tab matches, accepts directories for
continued navigation, filters files to `.kdbx`, and displays matching paths
below the input.

## Work sequence

1. Add tab-domain types and active-tab accessors; migrate initialization and
   auto-lock cleanup.
2. Migrate index and edit behavior, including save paths and per-tab filters.
3. Migrate settings, import/export, password changes, and clipboard commands.
4. Implement open-database modal state, path/password/keyfile handling, and
   atomic tab insertion.
5. Implement tab navigation, close confirmation, duplicate-path handling, and
   dirty-state behavior.
6. Add shared `Tabs` rendering and update all unlocked-screen layouts/help.
7. Add tests, run formatting/lints/tests, and manually exercise two vaults with
   password-only and keyfile-protected combinations.

## Files likely to change

- `src/app.rs`: tab collection, active index, modal state, lifecycle cleanup.
- `src/input/login.rs`: construct the initial `VaultTab`.
- `src/input/index.rs` and `src/input/command.rs`: tab/open navigation and
  active-session routing.
- `src/input/edit.rs`: per-tab entry mutation and save target.
- `src/input/settings.rs`: per-tab database operations and modal boundaries.
- `src/ui/index.rs`, `src/ui/edit.rs`, `src/ui/settings.rs`: shared tab layout,
  titles, and help text.
- `src/ui/login.rs`: open-vault modal if the final UX renders it there.
- `src/ui/mod.rs` or `src/ui/tabs.rs`: Ratatui `Tabs` renderer.
- `src/main.rs`: enable and disable Crossterm mouse capture.
- `src/db.rs`: only if unlock/path normalization helpers need extraction.
- `src/input/mod.rs`: new module registration if tab/open handling is split out.

## Testing strategy

### Unit tests

- tab insertion selects the new tab only after successful unlock;
- failed unlock leaves the tab vector, active index, and current session intact;
- equivalent paths are detected as duplicates;
- previous/next navigation wraps and close adjusts the active index;
- closing the final tab returns to the expected login state;
- auto-lock clears every tab's decrypted entries, keys, and sensitive buffers;
- per-tab query/filter/table selection is independent.

### Persistence and routing tests

Use temporary KDBX files created through existing database helpers. Open two
files with distinct entries, edit/save one, and assert that only its file
changes. Exercise import/export, copy-source lookup, and password-change
selection against the active tab. Include at least one password-plus-keyfile
vault and verify that an incorrect/missing keyfile does not create a tab.

### UI/manual checks

- Confirm `Tabs` renders in normal and slim modes with one, two, duplicate-name,
  and many tabs.
- Confirm narrow terminal dimensions do not panic and keep the active tab
  visible.
- Confirm help text exposes the new shortcuts without removing existing ones.
- Open, switch, edit, save, close, and auto-lock two live vaults in sequence.

## Risks and mitigations

- **State migration omissions:** keep vault-bound state inside one `VaultTab`
  and remove singleton aliases as each handler is migrated.
- **Wrong-vault writes:** require save/import/export/password-change code to use
  one active-session accessor and add routing tests with distinct temp files.
- **Sensitive data retained after lock:** centralize tab clearing and test all
  tabs, including inactive ones and modal buffers.
- **Tab switching during modal work:** disable switching while an operation is
  in progress or scope the complete modal state per tab; enforce this in input
  dispatch.
- **Long or duplicate labels:** derive stable display labels from canonical
  paths and test collision/narrow-layout behavior.
- **Keyfile ambiguity:** collect keyfile choice per opened vault and never
  silently inherit the default tab's configured keyfile.
- **Ratatui layout regressions:** use shared constraints and render tests or
  manual checks at minimum and narrow terminal widths.
- **Mouse hit-testing drift:** keep tab label/divider spacing in one renderer
  helper and use the same spacing assumptions for click mapping.

## Definition of done

The acceptance criteria in the specification are met, all existing tests pass,
new tests cover tab lifecycle and active-vault routing, and the two dated docs
remain accurate after the final key assignments and state-model decisions are
implemented.
