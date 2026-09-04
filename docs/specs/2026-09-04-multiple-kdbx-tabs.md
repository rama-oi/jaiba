# Specification: Open Multiple KDBX Files in Tabs

## Summary

Enhance Jaiba so that several `.kdbx` vaults can remain unlocked and usable in
parallel, with one vault shown at a time through a tab bar. The interaction
should be familiar to users of KeePassXC's multiple-database workflow and
should use Ratatui's `Tabs` widget for the visible tab strip.

## Problem

Jaiba currently has one global unlocked database (`kdbx`), one database key,
one entry list, and one configured database path. Opening another vault is only
possible by replacing or importing into the current vault. Users who compare,
copy, or maintain entries across vaults must leave Jaiba and reopen the other
file, losing the current working context.

## Goals

- Keep multiple unlocked KDBX vaults open in one Jaiba process.
- Make the active vault obvious with a Ratatui `Tabs` widget.
- Allow users to open another `.kdbx` file while retaining the current tabs.
- Allow users to select a tab with a left mouse click.
- Switch tabs without losing each vault's search, selection, and edit context.
- Route all reads, writes, imports, exports, password changes, and clipboard
  actions to the active vault.
- Preserve existing password and optional keyfile semantics.
- Ensure auto-lock clears decrypted data for every open vault.

## Non-goals

- Synchronizing, merging, or copying entries automatically between vaults.
- Sharing or caching a master password between tabs.
- Changing the on-disk KDBX format or the existing config file format.
- Opening arbitrary non-KDBX files as tabs; the existing import flow remains a
  separate operation.
- Background file watching or multi-process conflict resolution.
- Making the configured `default_database` a persistent list of open tabs.

## User experience

### Tab strip

When at least one vault is unlocked, the index, edit, and settings screens show
a tab strip at the top of the content area. Each tab is labelled with a concise
vault name, preferably the file name without its `.kdbx` suffix. If two files
have the same name, labels must be disambiguated without hiding which file is
active (for example, by adding a parent-directory suffix or an ordinal).

The active tab uses Ratatui `Tabs` selection styling. The tab strip is shown
only when at least two vaults are open. When visible, left-clicking a tab
selects it. The tab strip must remain
usable when the number of tabs exceeds the terminal width; labels may be
shortened or scrolled, but the active tab must remain identifiable.

Tab contents are isolated. Switching tabs changes the visible entries and
active database, but does not close, relock, or overwrite any other tab.

### Opening a vault

If `default_database` is not configured, Jaiba opens the database picker as
the initial screen. Canceling it returns to the normal login/create flow. If a
default path is configured but missing, the existing create-database flow is
retained.

Introduce a keyboard shortcut from the unlocked UI for “open database” (the
exact key is an implementation decision, but it must be documented in the
help/footer and must not conflict with existing shortcuts). The flow should:

1. Prompt for a path to a `.kdbx` file.
2. Prompt for that vault's master password.
3. Use the configured keyfile only when explicitly applicable to the opened
   vault; the implementation must not silently apply the current tab's keyfile
   to an unrelated vault.
4. Unlock and validate the file before adding a tab.
5. Select the new tab only after a successful unlock.

Canceling any prompt leaves existing tabs unchanged. A wrong password, missing
file, invalid KDBX, or keyfile failure shows an error and does not create a
partially initialized tab. Reopening an already-open canonical path should
either select the existing tab or show a clear “already open” message; it must
not create ambiguous duplicate tabs.

The path field supports custom `Tab` completion for directories and `.kdbx`
files. Shared prefixes are completed first; repeated `Tab` cycles through
multiple matches. A completed directory receives a trailing separator, and
matching candidates are shown below the field. Typing or deleting resets the
completion candidates.

The initial configured database continues to open through the current login
flow. Opening another vault must not change `default_database` unless the user
explicitly edits that setting.

### Switching and closing

Add shortcuts for moving to the previous and next tab, with wraparound, and a
shortcut for selecting a tab by number if practical. The plan should choose
keys that do not collide with current Ctrl shortcuts and should update the
footer help in normal and slim modes.

Provide a close-tab action. If the active tab has unsaved changes, closing it
must ask for confirmation and offer cancel. Closing the final tab returns to a
login/no-database state or exits according to the existing application
behavior; it must never discard a dirty vault silently. Closing a non-active
tab must preserve the active tab and adjust the selected index safely.

### Existing operations

All existing operations that act on the current vault must use the active tab:

- entry search, preview, create, edit, and delete;
- save and change-master-password;
- import and export;
- copy username, password, TOTP, and URL;
- entry reuse/duplicate warnings.

Settings for global behavior (theme, auto-lock duration, clipboard timeout) stay
global. Database path and keyfile settings must be clearly scoped: they describe
the default-login vault, while opening another tab collects per-open-vault
credentials in its own flow.

### Locking and sensitive data

Auto-lock must clear every tab's decrypted database, database key, entries,
pending edit buffers, and any per-tab password material before returning to the
login state. A tab switch must not put master passwords into the tab label,
status text, or logs. Errors may identify a path, but must not expose password
input or keyfile contents.

## Requirements

- The tab bar is rendered with `ratatui::widgets::Tabs`, not a hand-built text
  approximation.
- The tab bar consumes no layout space when fewer than two tabs are open.
- With mouse capture enabled, left-clicking a tab title selects that tab;
  clicks outside tab titles have no effect.
- Each tab owns its database handle, key, canonical path, display label, entry
  cache, filter/query state, table selection, and edit-related state needed to
  resume safely.
- Only the active tab is mutated by active-screen input handlers.
- Save operations use the active tab's path and key.
- A failed open is atomic from the user's perspective: existing tabs and the
  active tab remain unchanged.
- Path comparison must handle equivalent paths sufficiently to prevent obvious
  duplicate opens (at minimum normalize `.`/`..` and expand `~`; canonicalize
  when the file exists).
- Existing single-vault workflows and configuration behavior continue to work.

## Acceptance criteria

1. Starting with the configured vault open, a user can invoke the new open
   action, unlock a second `.kdbx`, and see two tabs rendered by `Tabs`.
2. Left-clicking a tab, or using the keyboard switching shortcuts, displays the
   correct vault entries and active label;
   changes in one vault do not appear in another.
3. Search text and selected entry state are retained independently per tab.
4. Editing an entry and saving writes only to the active vault's path and key.
5. Import, export, copy actions, and password change operate on the active
   vault and have tests covering their target selection.
6. Cancelled or failed opens do not add a tab or disturb the current tab.
7. Duplicate open paths are handled explicitly and do not create two tabs for
   the same file.
8. Closing a clean tab works; closing a dirty tab requires confirmation; the
   selected index remains valid after removal.
9. Auto-lock removes sensitive state from all tabs and requires the existing
   login flow before a vault can be used again.
10. Normal and slim help text documents opening, switching, and closing tabs,
    and existing shortcuts remain available.
11. With zero or one open tab, no tab bar is visible and no empty tab-strip
    space is reserved.
12. When no default database is configured, startup opens the database picker;
    its path field supports custom `Tab` completion.
13. Existing tests pass, and new unit/integration tests cover tab management,
    active-tab routing, open failure atomicity, and lock cleanup.

## Open decisions for planning

- Exact key assignments for open, next/previous, numbered selection, and close.
- Whether a per-tab keyfile path is collected in the open flow or selected by a
  small follow-up prompt.
- Whether edit/settings screens show the tab strip directly or only the index
  screen does; the preferred behavior is to show it on every unlocked screen.
- The precise dirty-state model for edits and pending import/password changes.
