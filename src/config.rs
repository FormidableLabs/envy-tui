use std::collections::HashMap;
use std::error::Error;
use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{de::Deserializer, Deserialize};

use crate::app::Action;

const CONFIG: &str = include_str!("../.config/config.yml");

#[derive(Clone, Debug, Default)]
pub struct Mapping(pub HashMap<KeyEvent, Action>);

impl<'de> Deserialize<'de> for Mapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let parsed_map = HashMap::<String, Action>::deserialize(deserializer)?;

        let keybindings = parsed_map
            .into_iter()
            .map(|(key_str, cmd)| (parse_key_event(&key_str).unwrap(), cmd))
            .collect();

        Ok(Mapping(keybindings))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mapping: Mapping,
}

pub fn parse(contents: &str) -> Result<Mapping, Box<dyn Error>> {
    let mapping = serde_yaml::from_str::<Mapping>(&contents)?;
    Ok(mapping)
}

pub fn load(path: &str) -> Result<Mapping, Box<dyn Error>> {
    let abs_path = fs::canonicalize(path)?;
    let contents = fs::read_to_string(abs_path)?;
    parse(&contents)
}

impl Config {
    pub fn new() -> Result<Config, Box<dyn Error>> {
        let default = parse(CONFIG)?;

        let mut cfg = Config { mapping: default };

        for file in &["config.yaml", "config.yml"] {
            match load(file) {
                Ok(right) => cfg.mapping.0.extend(right.0.into_iter()),
                Err(e) => println!("failed to load file: {}, err: {}", file, e),
            }
        }

        Ok(cfg)
    }
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let raw_lower = raw.to_ascii_lowercase();
    let modifiers = KeyModifiers::empty();
    parse_key_code_with_modifiers(&raw_lower, modifiers)
}

fn parse_key_code_with_modifiers(
    raw: &str,
    mut modifiers: KeyModifiers,
) -> Result<KeyEvent, String> {
    let c = match raw {
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backtab" => {
            modifiers.insert(KeyModifiers::SHIFT);
            KeyCode::BackTab
        }
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        "space" => KeyCode::Char(' '),
        "hyphen" => KeyCode::Char('-'),
        "minus" => KeyCode::Char('-'),
        "tab" => KeyCode::Tab,
        c if c.len() == 1 => {
            let mut c = c.chars().next().unwrap();
            if modifiers.contains(KeyModifiers::SHIFT) {
                c = c.to_ascii_uppercase();
            }
            KeyCode::Char(c)
        }
        _ => return Err(format!("Unable to parse {raw}")),
    };
    Ok(KeyEvent::new(c, modifiers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let c = Config::new();
        let k = &parse_key_event("q").unwrap();

        assert_eq!(c.mapping.get(k).unwrap(), &Action::Quit);
    }
}
