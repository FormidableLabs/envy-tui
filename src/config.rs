use std::collections::HashMap;
use std::error::Error;
use std::fs;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::app::{Action, Mapping};

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mapping: Mapping,
}

pub fn load(path: &str) -> Result<Mapping, Box<dyn Error>> {
    let abs_path = fs::canonicalize(path)?;
    let file_contents = fs::read_to_string(abs_path)?;
    let mapping = serde_yaml::from_str::<Mapping>(&file_contents)?;
    Ok(mapping)
}

impl Config {
    pub fn new() -> Config {
        let mut cfg = Config { mapping: default() };
        let config_files = ["config.yaml", "config.yml"];

        for file in &config_files {
            match load(file) {
                Ok(right) => cfg.mapping.extend(right.into_iter()),
                _ => {}
            }
        }

        cfg
    }
}

fn default() -> Mapping {
    HashMap::from([
        (
            default_event(KeyCode::Char('h')),
            Action::NavigateLeft(default_event(KeyCode::Char('h'))),
        ),
        (
            default_event(KeyCode::Down),
            Action::NavigateDown(default_event(KeyCode::Down)),
        ),
        (
            default_event(KeyCode::Char('j')),
            Action::NavigateDown(default_event(KeyCode::Char('j'))),
        ),
        (
            default_event(KeyCode::Up),
            Action::NavigateUp(default_event(KeyCode::Up)),
        ),
        (
            default_event(KeyCode::Char('k')),
            Action::NavigateUp(default_event(KeyCode::Char('k'))),
        ),
        (
            default_event(KeyCode::Char('l')),
            Action::NavigateRight(default_event(KeyCode::Char('l'))),
        ),
        (default_event(KeyCode::Char('>')), Action::GoToEnd),
        (default_event(KeyCode::Char('K')), Action::GoToEnd),
        (default_event(KeyCode::Char('<')), Action::GoToStart),
        (default_event(KeyCode::Char('J')), Action::GoToStart),
        (default_event(KeyCode::Char('q')), Action::Quit),
        (default_event(KeyCode::Tab), Action::NextSection),
        (default_event(KeyCode::BackTab), Action::PreviousSection),
        (default_event(KeyCode::Char('y')), Action::CopyToClipBoard),
        (default_event(KeyCode::Char('/')), Action::NewSearch),
        (default_event(KeyCode::Esc), Action::FocusOnTraces),
    ])
}

fn default_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
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
