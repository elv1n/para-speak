use crate::patterns::{
    combo::ComboPattern, double::DoubleTapPattern, single::SingleKeyPattern, Pattern,
};
use crate::types::ShortcutAction;
use rdev::Key;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid pattern format: {0}")]
    InvalidFormat(String),
    #[error("Unknown key: {0}")]
    UnknownKey(String),
    #[error("Invalid timeout value: {0}")]
    InvalidTimeout(String),
    #[error("Empty pattern")]
    EmptyPattern,
}

fn parse_single_key(
    input: &str,
    action: ShortcutAction,
) -> Result<Vec<Box<dyn Pattern>>, ParseError> {
    if let Some((left, right)) = parse_ambiguous_modifier_key(input) {
        Ok(vec![
            Box::new(SingleKeyPattern::new(left, action)),
            Box::new(SingleKeyPattern::new(right, action)),
        ])
    } else {
        let key = parse_key_name(input)?;
        Ok(vec![Box::new(SingleKeyPattern::new(key, action))])
    }
}

fn parse_combo(input: &str, action: ShortcutAction) -> Result<Vec<Box<dyn Pattern>>, ParseError> {
    let key_names: Vec<&str> = input.split('+').map(|s| s.trim()).collect();

    if key_names.is_empty() {
        return Err(ParseError::EmptyPattern);
    }

    let mut sequences = vec![vec![]];

    for key_name in key_names {
        if let Some((left, right)) = parse_ambiguous_modifier_key(key_name) {
            let mut new_sequences = Vec::new();
            for sequence in sequences {
                let mut left_seq = sequence.clone();
                left_seq.push(left);
                new_sequences.push(left_seq);

                let mut right_seq = sequence;
                right_seq.push(right);
                new_sequences.push(right_seq);
            }
            sequences = new_sequences;
        } else {
            let key = parse_key_name(key_name)?;
            for sequence in &mut sequences {
                sequence.push(key);
            }
        }
    }

    let mut result = Vec::new();
    for sequence in sequences {
        result.push(Box::new(ComboPattern::new(sequence, action)) as Box<dyn Pattern>);
    }

    Ok(result)
}

fn parse_double_tap(
    input: &str,
    action: ShortcutAction,
) -> Result<Vec<Box<dyn Pattern>>, ParseError> {
    let content = input
        .strip_prefix("double(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| ParseError::InvalidFormat(input.to_string()))?;

    let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(ParseError::EmptyPattern);
    }

    let timeout_ms = if parts.len() > 1 {
        parts[1]
            .parse::<u64>()
            .map_err(|_| ParseError::InvalidTimeout(parts[1].to_string()))?
    } else {
        300
    };

    if let Some((left, right)) = parse_ambiguous_modifier_key(parts[0]) {
        Ok(vec![
            Box::new(DoubleTapPattern::new(left, timeout_ms, action)),
            Box::new(DoubleTapPattern::new(right, timeout_ms, action)),
        ])
    } else {
        let key = parse_key_name(parts[0])?;
        Ok(vec![Box::new(DoubleTapPattern::new(
            key, timeout_ms, action,
        ))])
    }
}

pub fn parse_pattern(
    input: &str,
    action: ShortcutAction,
) -> Result<Vec<Box<dyn Pattern>>, ParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ParseError::EmptyPattern);
    }

    if input.starts_with("double(") && input.ends_with(')') {
        parse_double_tap(input, action)
    } else if input.contains('+') {
        parse_combo(input, action)
    } else {
        parse_single_key(input, action)
    }
}

fn parse_ambiguous_modifier_key(name: &str) -> Option<(Key, Key)> {
    match name {
        "Control" | "Ctrl" => Some((Key::ControlLeft, Key::ControlRight)),
        "Shift" => Some((Key::ShiftLeft, Key::ShiftRight)),
        "Alt" | "Option" => Some((Key::Alt, Key::AltGr)),
        "Meta" | "Cmd" | "Command" | "Win" | "Windows" | "Super" => {
            Some((Key::MetaLeft, Key::MetaRight))
        }
        _ => None,
    }
}

fn parse_key_name(name: &str) -> Result<Key, ParseError> {
    match name {
        "CommandLeft" | "MetaLeft" => Ok(Key::MetaLeft),
        "CommandRight" | "MetaRight" => Ok(Key::MetaRight),
        "ControlLeft" => Ok(Key::ControlLeft),
        "ControlRight" => Ok(Key::ControlRight),
        "ShiftLeft" => Ok(Key::ShiftLeft),
        "ShiftRight" => Ok(Key::ShiftRight),
        "AltLeft" => Ok(Key::Alt),
        "AltRight" => Ok(Key::AltGr),

        "KeyA" | "A" => Ok(Key::KeyA),
        "KeyB" | "B" => Ok(Key::KeyB),
        "KeyC" | "C" => Ok(Key::KeyC),
        "KeyD" | "D" => Ok(Key::KeyD),
        "KeyE" | "E" => Ok(Key::KeyE),
        "KeyF" | "F" => Ok(Key::KeyF),
        "KeyG" | "G" => Ok(Key::KeyG),
        "KeyH" | "H" => Ok(Key::KeyH),
        "KeyI" | "I" => Ok(Key::KeyI),
        "KeyJ" | "J" => Ok(Key::KeyJ),
        "KeyK" | "K" => Ok(Key::KeyK),
        "KeyL" | "L" => Ok(Key::KeyL),
        "KeyM" | "M" => Ok(Key::KeyM),
        "KeyN" | "N" => Ok(Key::KeyN),
        "KeyO" | "O" => Ok(Key::KeyO),
        "KeyP" | "P" => Ok(Key::KeyP),
        "KeyQ" | "Q" => Ok(Key::KeyQ),
        "KeyR" | "R" => Ok(Key::KeyR),
        "KeyS" | "S" => Ok(Key::KeyS),
        "KeyT" | "T" => Ok(Key::KeyT),
        "KeyU" | "U" => Ok(Key::KeyU),
        "KeyV" | "V" => Ok(Key::KeyV),
        "KeyW" | "W" => Ok(Key::KeyW),
        "KeyX" | "X" => Ok(Key::KeyX),
        "KeyY" | "Y" => Ok(Key::KeyY),
        "KeyZ" | "Z" => Ok(Key::KeyZ),

        "Escape" | "Esc" => Ok(Key::Escape),
        "Space" => Ok(Key::Space),
        "Return" | "Enter" => Ok(Key::Return),
        "Tab" => Ok(Key::Tab),
        "Backspace" => Ok(Key::Backspace),
        "Delete" => Ok(Key::Delete),

        "F1" => Ok(Key::F1),
        "F2" => Ok(Key::F2),
        "F3" => Ok(Key::F3),
        "F4" => Ok(Key::F4),
        "F5" => Ok(Key::F5),
        "F6" => Ok(Key::F6),
        "F7" => Ok(Key::F7),
        "F8" => Ok(Key::F8),
        "F9" => Ok(Key::F9),
        "F10" => Ok(Key::F10),
        "F11" => Ok(Key::F11),
        "F12" => Ok(Key::F12),

        _ => Err(ParseError::UnknownKey(name.to_string())),
    }
}

pub fn parse_multiple_patterns(
    input: &str,
    action: ShortcutAction,
) -> Result<Vec<Box<dyn Pattern>>, ParseError> {
    let patterns: Vec<&str> = input.split(';').map(|s| s.trim()).collect();
    let mut result = Vec::new();

    for pattern_str in patterns {
        if !pattern_str.is_empty() {
            let parsed_patterns = parse_pattern(pattern_str, action)?;
            result.extend(parsed_patterns);
        }
    }

    Ok(result)
}
