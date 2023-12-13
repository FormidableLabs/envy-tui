use std::collections::HashMap;
use std::error::Error;
use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;
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
            .map(|(key, cmd)| (parse_key_event(&key).unwrap(), cmd))
            .collect();

        Ok(Mapping(keybindings))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mapping: Mapping,
    #[serde(default)]
    pub colors: Colors,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Colors {
    pub surface: SurfaceColors,
    pub text: TextColors,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SurfaceColors {
    pub bg: Color,
    pub selected: Color,
    pub unselected: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub null: Color,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TextColors {
    pub selected: Color,
    pub unselected: Color,
    pub default: Color,
    pub accent_1: Color,
    pub accent_2: Color,
}

pub fn parse(contents: &str) -> Result<Config, Box<dyn Error>> {
    let config = serde_yaml::from_str::<Config>(contents)?;
    Ok(config)
}

pub fn load(path: &str) -> Result<Config, Box<dyn Error>> {
    let abs_path = fs::canonicalize(path)?;
    let contents = fs::read_to_string(abs_path)?;
    parse(&contents)
}

impl Config {
    pub fn new() -> Result<Config, Box<dyn Error>> {
        let default = parse(CONFIG)?;

        let mut cfg = default;

        for file in &["config.yaml", "config.yml"] {
            match load(file) {
                Ok(right) => {
                    cfg.mapping.0.extend(right.mapping.0.into_iter());
                    // cfg.colors.extend(right.colors.0.into_iter())
                }
                Err(e) => println!("failed to load file: {}, err: {}", file, e),
            }
        }

        Ok(cfg)
    }
}

fn parse_key_event(raw: &str) -> Result<KeyEvent, String> {
    let modifiers = KeyModifiers::empty();
    parse_key_code_with_modifiers(&raw, modifiers)
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
    use std::error::Error;

    #[test]
    fn test_config() -> Result<(), Box<dyn Error>> {
        let c = Config::new()?;
        let k = &parse_key_event("q")?;

        assert_eq!(c.mapping.0.get(k).unwrap(), &Action::Quit);

        Ok(())
    }

    #[test]
    fn test_parse_color_rgb() {
        let color = parse_color("rgb(255,255,255)");
        assert_eq!(color, Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn test_parse_color_named() {
        let color = parse_color("black");
        assert_eq!(color, Some(Color::Indexed(0)));
    }

    #[test]
    fn test_parse_color_unknown() {
        let color = parse_color("unknown");
        assert_eq!(color, None);
    }
}
