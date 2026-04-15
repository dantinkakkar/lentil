use std::{env, fs, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LentilConfig {
    pub clear_and_stay_open: bool,
    pub suggest_idea_without_due: bool,
    pub require_priority_with_due: bool,
}

impl Default for LentilConfig {
    fn default() -> Self {
        Self {
            clear_and_stay_open: false,
            suggest_idea_without_due: true,
            require_priority_with_due: true,
        }
    }
}

impl LentilConfig {
    pub fn load() -> Self {
        let Some(path) = default_config_path() else {
            return Self::default();
        };

        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        parse_config(&contents)
    }
}

pub fn default_config_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    let mut path = PathBuf::from(home);
    path.push(".config");
    path.push("lentil");
    path.push("config.toml");
    Some(path)
}

fn parse_config(contents: &str) -> LentilConfig {
    let mut config = LentilConfig::default();

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = raw_value.trim();
        let parsed_bool = match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };

        match (key, parsed_bool) {
            ("clear_and_stay_open", Some(parsed)) => config.clear_and_stay_open = parsed,
            ("suggest_idea_without_due", Some(parsed)) => {
                config.suggest_idea_without_due = parsed;
            }
            ("require_priority_with_due", Some(parsed)) => {
                config.require_priority_with_due = parsed;
            }
            _ => {}
        }
    }

    config
}
