use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub font: FontSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FontSettings {
    pub family: Option<String>,
    pub size: Option<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: FontSettings::default(),
        }
    }
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: None,
            size: None,
        }
    }
}

impl FontSettings {
    pub fn from_guifont(guifont: &str) -> Option<Self> {
        if guifont.is_empty() {
            return None;
        }

        // Handle list of fonts (take first)
        let first_font = guifont.split(',').next().unwrap_or(guifont);

        // Try parsing "Family:hSize" (macOS/Windows style)
        if let Some((family, size_str)) = first_font.rsplit_once(":h") {
            let size = size_str.parse::<f32>().ok();
            let family = family.replace("\\ ", " ");
            return Some(Self {
                family: Some(family),
                size,
            });
        }

        // Try parsing "Family Size" (GTK/X11 style)
        if let Some((family, size_str)) = first_font.rsplit_once(' ') {
            if let Ok(size) = size_str.parse::<f32>() {
                let family = family.replace("\\ ", " ");
                return Some(Self {
                    family: Some(family),
                    size: Some(size),
                });
            }
        }

        // Just family
        let family = first_font.replace("\\ ", " ");
        Some(Self {
            family: Some(family),
            size: None,
        })
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = match config_file_path() {
            Some(path) => path,
            None => return Config::default(),
        };

        if !config_path.exists() {
            return Config::default();
        }

        let content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to read config file: {}", e);
                return Config::default();
            }
        };

        match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                log::warn!("Failed to parse config file: {}", e);
                Config::default()
            }
        }
    }
}

fn config_file_path() -> Option<PathBuf> {
    if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME") {
        Some(
            PathBuf::from(config_dir)
                .join("gui-nvim")
                .join("config.toml"),
        )
    } else if let Some(home) = std::env::var_os("HOME") {
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("gui-nvim")
                .join("config.toml"),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.font.family, None);
        assert_eq!(config.font.size, None);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [font]
            family = "Fira Code"
            size = 16.0
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.font.family.as_deref(), Some("Fira Code"));
        assert_eq!(config.font.size, Some(16.0));
    }

    #[test]
    fn test_parse_partial_config() {
        let toml = r#"
            [font]
            size = 20.0
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.font.family, None);
        assert_eq!(config.font.size, Some(20.0));
    }

    #[test]
    fn test_from_guifont_simple() {
        let settings = FontSettings::from_guifont("Fira Code:h14").unwrap();
        assert_eq!(settings.family.as_deref(), Some("Fira Code"));
        assert_eq!(settings.size, Some(14.0));
    }

    #[test]
    fn test_from_guifont_escaped_space() {
        let settings = FontSettings::from_guifont("Fira\\ Code:h14").unwrap();
        assert_eq!(settings.family.as_deref(), Some("Fira Code"));
        assert_eq!(settings.size, Some(14.0));
    }

    #[test]
    fn test_from_guifont_gtk_style() {
        let settings = FontSettings::from_guifont("Fira Code 14").unwrap();
        assert_eq!(settings.family.as_deref(), Some("Fira Code"));
        assert_eq!(settings.size, Some(14.0));
    }

    #[test]
    fn test_from_guifont_multiple() {
        let settings = FontSettings::from_guifont("Fira Code:h14,Monospace:h12").unwrap();
        assert_eq!(settings.family.as_deref(), Some("Fira Code"));
        assert_eq!(settings.size, Some(14.0));
    }

    #[test]
    fn test_from_guifont_no_size() {
        let settings = FontSettings::from_guifont("Fira Code").unwrap();
        assert_eq!(settings.family.as_deref(), Some("Fira Code"));
        assert_eq!(settings.size, None);
    }
}
