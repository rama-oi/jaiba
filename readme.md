# Jaiba

![Login Screen](https://raw.githubusercontent.com/pomboverso/jaiba/HEAD/assets/screenshots/1.png)

Jaiba is a terminal password manager built on top of the [KeePass](https://keepass.info/)
(`.kdbx`) file format, so your vault stays compatible with the wider KeePass ecosystem.
It's fast to open, keyboard-driven, and stores only what it needs:

- Name
- Username
- Password
- TOTP (two-factor codes)
- URL
- Notes

## Features

- **KDBX4 vaults**: reads and writes standard `.kdbx` files, so you can open the same vault in KeePassXC, KeePassDX, or any other compatible client.
- **Optional keyfile support**: unlocks vaults protected by a master password plus a KeePass keyfile while preserving password-only vault support.
- **Fuzzy search**: start typing on the index screen to filter entries by name or user.
- **TOTP codes**: generates live 2FA codes from a stored seed or `otpauth://` URI and copies them straight to your clipboard.
- **Reuse warnings**: flags entries that share a password or username with another entry, so you can spot weak spots at a glance.
- **Auto-lock & clipboard clearing**: the vault locks itself after a period of inactivity, and anything copied to the clipboard is cleared automatically.
- **Themeable**: ships with a default color scheme and supports custom themes.
- **Change your master password**: from Settings, without needing to touch a file manager or another app.
- **Import from other vaults or exports**: pull entries in from another `.kdbx` file, or from a CSV/JSON export produced by another password manager.

## Screenshots

![Index Screen](https://raw.githubusercontent.com/pomboverso/jaiba/HEAD/assets/screenshots/2.png)

![Edit Screen](https://raw.githubusercontent.com/pomboverso/jaiba/HEAD/assets/screenshots/4.png)

## Installing

### Prebuilt packages (recommended)

Grab the file for your system from the [latest release](https://github.com/pomboverso/jaiba/releases/latest).


### From crates.io

```sh
cargo install jaiba
```

## Getting started

On first launch, if no vault is found at the configured path, Jaiba will:

- Prompt you to create a new database
- You'll be asked to set a master password

Once unlocked, you land on the index screen, where you can search, browse, and open entries.

To open an existing vault, set `default_database` in the config. If the vault also requires a keyfile, set `keyfile` as well; Jaiba will still prompt for and require the master password:

```toml
default_database = "~/Documents/passwords.kdbx"
keyfile = "~/Documents/passwords.keyx"
```

## Keybindings

### Index screen

| Key       | Action                        |
| --------- | ----------------------------- |
| `↑` / `↓` | Move selection                |
| type      | Filter entries by name / user |
| `Enter`   | Open the selected entry       |
| `:`       | Enter command mode            |

### Commands (after pressing `:`)

| Command | Action                 |
| ------- | ---------------------- |
| `:u`    | Copy username          |
| `:p`    | Copy password          |
| `:t`    | Copy current TOTP code |
| `:r`    | Copy URL               |
| `:a`    | Add a new entry        |
| `:s`    | Open Settings          |
| `:q`    | Quit                   |

Commands are case-insensitive, so `:S` works the same as `:s`.

### Settings screen

| Key     | Action                               |
| ------- | ------------------------------------ |
| `↑`/`↓` | Navigate rows                        |
| `Enter` | Edit the selected field / pick theme |
| `Esc`   | Back to the index screen             |

From Settings you can change the default vault path, optional keyfile path, auto-lock timeout, clipboard timeout, active theme, and the vault's master password (you'll be asked for the current password first, then the new one twice).

## Configuration

Jaiba reads its config from `~/.config/jaiba/config.toml`:

```toml
default_database = "~/.local/share/jaiba/default.kdbx"
keyfile = "~/.local/share/jaiba/default.keyx" # optional; omit for password-only vaults
auto_lock = 300           # seconds of inactivity before locking
clipboard_timeout = 15    # seconds before a copied value is cleared
theme = "catppuccin-mocha"
```

All fields are optional; missing ones fall back to sane defaults. In particular, omitting `keyfile` keeps the original password-only behavior. Paths support `~` expansion. You can also edit these values live from the Settings screen instead of hand-editing the file. See [`config.example.toml`](config.example.toml) for a copyable example.

## Themes

Drop `.toml` theme files into `~/.config/jaiba/themes/`. Each one looks like:

```toml
// catppuccin_mocha.toml
name = "catppuccin-mocha"

[colors]
background   = "#1e1e2e"
text         = "#cdd6f4"
border       = "#45475a"
header       = "#9399b2"
accent       = "#cba6f7"
warning      = "#f9e2af"
error        = "#f38ba8"
selection_fg = "#1e1e2e"
selection_bg = "#89b4fa"
claws        = "#437db6"
claws_light  = "#64a0d2"
claws_shadow = "#2d5c91"
shell        = "#567468"
shell_light  = "#789488"
shell_shadow = "#3a524a"
```

Pick one up from Settings → Theme, or set the `theme` key in `config.toml` directly.

## Security notes

- Vaults are standard KDBX4 files, encrypted with the master password you set and, when configured, the keyfile. Jaiba never treats the keyfile as a replacement for the password.
- Jaiba reads the configured keyfile when unlocking. A missing, unreadable, empty, or incorrect keyfile produces an explicit error; only its path is stored in the config, never its contents.
- Changing the master password re-encrypts the whole vault in place, and requires entering the _current_ password first. For a keyfile-protected vault, the configured keyfile remains part of the new composite key.
- The clipboard is cleared automatically after `clipboard_timeout` seconds, but only if it still holds the value Jaiba copied (so it won't stomp on something else you copied in the meantime).
- The app locks itself after `auto_lock` seconds of inactivity, clearing decrypted entries and the master password from memory.

## Releasing / packaging

For maintainers cutting a release — builds the `.deb`, `.rpm`, and `.AppImage`, dropping everything in `./dist/`:

```sh
./deploy.sh
```

Run `./deploy.sh` with no arguments to build everything, or `deb` / `rpm` / `appimage` to build just one. See the comments at the top of `deploy.sh` for per-target requirements.

## License

GPL-3.0-or-later
