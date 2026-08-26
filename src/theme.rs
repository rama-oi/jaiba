use std::fs;

use anyhow::Context;
use ratatui::style::Color;
use serde::Deserialize;

use crate::util::expand_tilde;

pub struct Theme {
    pub background: Color,
    pub text: Color,
    pub warning: Color,
    pub error: Color,
    // pub success: Color,
    pub border: Color,
    pub header: Color,
    pub accent: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,

    pub claws: Color,
    pub claws_light: Color,
    pub claws_shadow: Color,
    pub shell: Color,
    pub shell_light: Color,
    pub shell_shadow: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),
            text: Color::Rgb(205, 214, 244),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            border: Color::Rgb(69, 71, 90),
            header: Color::Rgb(147, 153, 178),
            accent: Color::Rgb(203, 166, 247),
            selection_fg: Color::Rgb(30, 30, 46),
            selection_bg: Color::Rgb(137, 180, 250),

            claws: Color::Rgb(67, 125, 182),
            claws_light: Color::Rgb(100, 160, 210),
            claws_shadow: Color::Rgb(45, 92, 145),
            shell: Color::Rgb(86, 116, 104),
            shell_light: Color::Rgb(120, 148, 136),
            shell_shadow: Color::Rgb(58, 82, 74),
        }
    }
}

#[derive(Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub colors: ThemeColors,
}

#[derive(Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub text: String,
    pub border: String,
    pub header: String,
    pub accent: String,
    pub warning: String,
    pub error: String,
    pub selection_fg: String,
    pub selection_bg: String,

    pub claws: String,
    pub claws_light: String,
    pub claws_shadow: String,
    pub shell: String,
    pub shell_light: String,
    pub shell_shadow: String,
}

impl TryFrom<ThemeConfig> for Theme {
    type Error = anyhow::Error;

    fn try_from(cfg: ThemeConfig) -> Result<Self, Self::Error> {
        Ok(Self {
            background: parse_hex(&cfg.colors.background)?,
            text: parse_hex(&cfg.colors.text)?,
            border: parse_hex(&cfg.colors.border)?,
            header: parse_hex(&cfg.colors.header)?,
            accent: parse_hex(&cfg.colors.accent)?,
            warning: parse_hex(&cfg.colors.warning)?,
            error: parse_hex(&cfg.colors.error)?,
            selection_fg: parse_hex(&cfg.colors.selection_fg)?,
            selection_bg: parse_hex(&cfg.colors.selection_bg)?,

            claws: parse_hex(&cfg.colors.claws)?,
            claws_light: parse_hex(&cfg.colors.claws_light)?,
            claws_shadow: parse_hex(&cfg.colors.claws_shadow)?,
            shell: parse_hex(&cfg.colors.shell)?,
            shell_light: parse_hex(&cfg.colors.shell_light)?,
            shell_shadow: parse_hex(&cfg.colors.shell_shadow)?,
        })
    }
}

fn parse_hex(hex: &str) -> anyhow::Result<Color> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);

    if hex.len() != 6 {
        anyhow::bail!("expected 6 hex digits");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(Color::Rgb(r, g, b))
}

const DEFAULT_THEMES: &[(&str, &str)] = &[
    (
        "catppuccin_latte.toml",
        include_str!("../themes/catppuccin_latte.toml"),
    ),
    (
        "catppuccin_mocha.toml",
        include_str!("../themes/catppuccin_mocha.toml"),
    ),
    ("dracula.toml", include_str!("../themes/dracula.toml")),
    ("mako.toml", include_str!("../themes/mako.toml")),
    (
        "melange_dark.toml",
        include_str!("../themes/melange_dark.toml"),
    ),
    ("pomboverso.toml", include_str!("../themes/pomboverso.toml")),
    ("rama.toml", include_str!("../themes/rama.toml")),
    ("teyin.toml", include_str!("../themes/teyin.toml")),
    (
        "tokyo_night.toml",
        include_str!("../themes/tokyo_night.toml"),
    ),
];

pub fn ensure_default_themes() -> anyhow::Result<()> {
    let dir = expand_tilde("~/.config/rama/themes");

    fs::create_dir_all(&dir).with_context(|| format!("couldn't create {}", dir.display()))?;

    for (filename, contents) in DEFAULT_THEMES {
        let path = dir.join(filename);

        if !path.exists() {
            fs::write(&path, contents)
                .with_context(|| format!("couldn't write {}", path.display()))?;
        }
    }

    Ok(())
}

pub fn load_theme(name: Option<&str>) -> anyhow::Result<Theme> {
    let name = name.ok_or_else(|| anyhow::anyhow!("no theme set in jaiba_config.toml"))?;

    let config = find_theme(name)?;
    Theme::try_from(config)
}

pub fn list_theme_names() -> anyhow::Result<Vec<String>> {
    let mut configs = read_theme_configs()?;
    configs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(configs.into_iter().map(|config| config.name).collect())
}

fn find_theme(name: &str) -> anyhow::Result<ThemeConfig> {
    let target = slugify(name);

    read_theme_configs()?
        .into_iter()
        .find(|config| slugify(&config.name) == target)
        .ok_or_else(|| anyhow::anyhow!("no theme named \"{name}\" found"))
}

pub fn slugify(name: &str) -> String {
    name.trim()
        .chars()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugifies_simple_names() {
        assert_eq!(slugify("Catppuccin Mocha"), "catppuccin-mocha");
        assert_eq!(slugify("Dracula"), "dracula");
    }

    #[test]
    fn slugify_collapses_whitespace() {
        assert_eq!(slugify("  Tokyo   Night  "), "tokyo-night");
    }

    #[test]
    fn slugify_is_idempotent() {
        assert_eq!(slugify("catppuccin-mocha"), "catppuccin-mocha");
    }
}

fn read_theme_configs() -> anyhow::Result<Vec<ThemeConfig>> {
    let dir = expand_tilde("~/.config/rama/themes");

    let entries = fs::read_dir(&dir).with_context(|| format!("couldn't read {}", dir.display()))?;

    let mut configs = Vec::new();

    for entry in entries {
        let path = entry?.path();

        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }

        let text = fs::read_to_string(&path)?;

        if let Ok(config) = toml::from_str::<ThemeConfig>(&text) {
            configs.push(config);
        }
    }

    Ok(configs)
}
