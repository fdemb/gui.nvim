use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub font: FontSettings,
    #[serde(default)]
    pub performance: PerformanceSettings,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VsyncMode {
    Enabled,
    Disabled,
    MailboxIfAvailable,
    /// macOS-only: Use CADisplayLink for frame synchronization.
    /// This provides proper vblank sync without relying on wgpu's vsync.
    /// Ignored on other platforms.
    DisplayLink,
}

#[allow(clippy::derivable_impls)]
impl Default for VsyncMode {
    fn default() -> Self {
        // macOS: Use CADisplayLink for proper frame sync (requires macOS 14+)
        // Other platforms: Use mailbox if available for lowest latency with vsync
        #[cfg(target_os = "macos")]
        {
            VsyncMode::DisplayLink
        }
        #[cfg(not(target_os = "macos"))]
        {
            VsyncMode::MailboxIfAvailable
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PerformanceSettings {
    #[serde(default)]
    pub vsync: VsyncMode,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FontSettings {
    pub family: Option<String>,
    pub size: Option<f32>,
}



impl FontSettings {
    pub fn from_guifont(guifont: &str) -> Option<Self> {
        if guifont.is_empty() {
            return None;
        }

        // Handle list of fonts (take first)
        let first_font = guifont.split(',').next().unwrap_or(guifont);

        if let Some((family, size_str)) = first_font.rsplit_once(":h") {
            let size = size_str.parse::<f32>().ok();
            let family = family.replace("\\ ", " ");
            return Some(Self {
                family: Some(family),
                size,
            });
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

/// Returns the gui-nvim config directory.
/// Location: `~/.config/gui-nvim/`
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(|d| PathBuf::from(d).join("gui-nvim"))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("gui-nvim"))
        })
}

fn config_file_path() -> Option<PathBuf> {
    config_dir().map(|p| p.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.font.family, None);
        assert_eq!(config.font.size, None);
        #[cfg(target_os = "macos")]
        assert_eq!(config.performance.vsync, VsyncMode::DisplayLink);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(config.performance.vsync, VsyncMode::MailboxIfAvailable);
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

    #[test]
    fn test_parse_performance_config() {
        let toml = r#"
            [performance]
            vsync = "enabled"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.performance.vsync, VsyncMode::Enabled);
    }
}
