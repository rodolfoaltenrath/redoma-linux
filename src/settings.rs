//! Opções escolhidas no assistente, em `~/.config/redoma/options`.
//! A existência do arquivo indica que a configuração foi concluída.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::desktop::config_home;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub hide_apps: bool,
    pub pin_dock: bool,
    pub autostart: bool,
    pub clipboard: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hide_apps: true,
            pin_dock: true,
            autostart: false,
            clipboard: false,
        }
    }
}

fn path() -> Option<PathBuf> {
    config_home().map(|config| config.join("redoma/options"))
}

impl Options {
    /// Opções salvas, ou `None` se a configuração nunca foi concluída.
    pub fn load() -> Option<Self> {
        fs::read_to_string(path()?)
            .ok()
            .map(|text| Self::parse(&text))
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.render())
    }

    fn parse(text: &str) -> Self {
        let mut options = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim() == "true";
            match key.trim() {
                "hide_apps" => options.hide_apps = value,
                "pin_dock" => options.pin_dock = value,
                "autostart" => options.autostart = value,
                "clipboard" => options.clipboard = value,
                _ => {}
            }
        }
        options
    }

    fn render(&self) -> String {
        format!(
            "hide_apps={}\npin_dock={}\nautostart={}\nclipboard={}\n",
            self.hide_apps, self.pin_dock, self.autostart, self.clipboard
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn round_trips_and_defaults_unknown_keys() {
        let options = Options {
            hide_apps: false,
            pin_dock: true,
            autostart: true,
            clipboard: false,
        };
        assert_eq!(Options::parse(&options.render()), options);
        assert_eq!(Options::parse("other=1\n"), Options::default());
    }
}
