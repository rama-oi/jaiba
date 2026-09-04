# Jaiba

![Login Screen](https://raw.githubusercontent.com/pomboverso/jaiba/HEAD/assets/screenshots/0.png)

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

### From crates.io

```sh
cargo install jaiba
```

## Getting started

On first launch, if no vault is found at the configured path, Jaiba will:

- Prompt you to create a new database
- You'll be asked to set a master password

Once unlocked, you land on the index screen, where you can search, browse, and open entries.

To open a vault for one session without changing the configured default, pass its path on the
command line:

```sh
jaiba --vault "~/Documents/work.kdbx"
```

The command-line path takes precedence over `default_database` and is not saved to the config.
For an existing `.kdbx` file, the path may also be supplied positionally:

```sh
jaiba "~/Documents/work.kdbx"
```

The positional form requires an existing `.kdbx` file and cannot be combined with `--vault`.

To open an existing vault, set `default_database` in the config. If the vault also requires a keyfile, set `keyfile` as well; Jaiba will still prompt for and require the master password:

```toml
default_database = "~/Documents/passwords.kdbx"
keyfile = "~/Documents/passwords.keyx"
```

## Configuration

Jaiba reads its config from `~/.config/rama/jaiba_config.toml`:

```toml
default_database = "~/.local/share/rama/default.kdbx"
keyfile = "~/.local/share/rama/default.keyx" # optional; omit for password-only vaults
auto_lock = 300           # seconds of inactivity before locking
clipboard_timeout = 15    # seconds before a copied value is cleared
theme = "catppuccin-mocha"
```

All fields are optional; missing ones fall back to sane defaults.

## Security notes

- Vaults are standard KDBX4 files, encrypted with the master password you set and, when configured, the keyfile. Jaiba never treats the keyfile as a replacement for the password.
- Jaiba reads the configured keyfile when unlocking. A missing, unreadable, empty, or incorrect keyfile produces an explicit error; only its path is stored in the config, never its contents.
- Changing the master password re-encrypts the whole vault in place, and requires entering the _current_ password first. For a keyfile-protected vault, the configured keyfile remains part of the new composite key.
- The clipboard is cleared automatically after `clipboard_timeout` seconds, but only if it still holds the value Jaiba copied (so it won't stomp on something else you copied in the meantime).
- The app locks itself after `auto_lock` seconds of inactivity, clearing decrypted entries and the master password from memory.
