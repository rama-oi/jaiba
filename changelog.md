## 6

- Change quit command from ^q to esc on index and login screen
- Themes are now using the same names as `Coqui` and `Caiman` `catppucin-mocha` instead of `Catppuccin Mocha`
- Ability to work in small windows so you can have `Jaiba` like a floating window
- Dropping support for packages, now the official intall is comming from cargo.
- Trimming the readme file so it can actually be diggested.

## 5

- Add version and help flag
- Reusing themes from all the rama apps instead of creating new ones under `.config/jaiba/themes` now they belong to `.config/rama/themes`
- Removing extra space around the crab so resizing works better
- Migrating from command mode to using ctrl+key
- Using sample file instead of generating a new one on first run

## 4

- Moving to an individual organization
- Fix clipboard on wayland

## 3

- Fix first-run database creation showing a false "failed" error when a vault already exists but Jaiba lost track of it
- Prevent uppercase to break the commands flow. Now is case insensitive.
- Add optional KeePass keyfile support
- Fix esc doing nothing on the "save changes?" prompt when leaving the edit screen. Now esc cancels and returns you to editing (previously only Y/N worked)

## 2

- Fix enter into edit mode will always save changes