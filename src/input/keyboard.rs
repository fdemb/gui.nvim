use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,
}

impl From<ModifiersState> for Modifiers {
    fn from(state: ModifiersState) -> Self {
        Self {
            ctrl: state.control_key(),
            shift: state.shift_key(),
            alt: state.alt_key(),
            logo: state.super_key(),
        }
    }
}

pub fn key_event_to_neovim(event: &KeyEvent, modifiers: &Modifiers) -> Option<String> {
    if event.state != ElementState::Pressed {
        return None;
    }

    let (key_str, is_special) = match &event.logical_key {
        Key::Named(named) => (named_key_to_str(*named)?, true),
        Key::Character(c) => {
            let s = c.as_str();
            if s.len() == 1 {
                let ch = s.chars().next().unwrap();
                if ch.is_control() {
                    return None;
                }
            }
            (s.to_string(), false)
        }
        Key::Unidentified(_) => return try_physical_key(&event.physical_key, modifiers),
        Key::Dead(_) => return None,
    };

    format_with_modifiers(&key_str, modifiers, is_special)
}

fn named_key_to_str(key: NamedKey) -> Option<String> {
    let s = match key {
        NamedKey::Enter => "CR",
        NamedKey::Tab => "Tab",
        NamedKey::Space => "Space",
        NamedKey::Backspace => "BS",
        NamedKey::Escape => "Esc",
        NamedKey::Delete => "Del",
        NamedKey::Insert => "Insert",
        NamedKey::Home => "Home",
        NamedKey::End => "End",
        NamedKey::PageUp => "PageUp",
        NamedKey::PageDown => "PageDown",
        NamedKey::ArrowUp => "Up",
        NamedKey::ArrowDown => "Down",
        NamedKey::ArrowLeft => "Left",
        NamedKey::ArrowRight => "Right",
        NamedKey::F1 => "F1",
        NamedKey::F2 => "F2",
        NamedKey::F3 => "F3",
        NamedKey::F4 => "F4",
        NamedKey::F5 => "F5",
        NamedKey::F6 => "F6",
        NamedKey::F7 => "F7",
        NamedKey::F8 => "F8",
        NamedKey::F9 => "F9",
        NamedKey::F10 => "F10",
        NamedKey::F11 => "F11",
        NamedKey::F12 => "F12",
        NamedKey::Help => "Help",
        NamedKey::Undo => "Undo",
        // Modifier keys should not produce output on their own
        NamedKey::Shift
        | NamedKey::Control
        | NamedKey::Alt
        | NamedKey::Super
        | NamedKey::Meta
        | NamedKey::Hyper => return None,
        // NumLock/CapsLock don't produce output
        NamedKey::NumLock | NamedKey::CapsLock | NamedKey::ScrollLock => return None,
        _ => return None,
    };
    Some(s.to_string())
}

fn is_special_key(key: &Key) -> bool {
    matches!(key, Key::Named(_))
}

fn try_physical_key(physical: &PhysicalKey, modifiers: &Modifiers) -> Option<String> {
    let key_str = match physical {
        PhysicalKey::Code(code) => physical_keycode_to_str(*code)?,
        PhysicalKey::Unidentified(_) => return None,
    };
    format_with_modifiers(&key_str, modifiers, true)
}

fn physical_keycode_to_str(code: KeyCode) -> Option<String> {
    let s = match code {
        KeyCode::Enter | KeyCode::NumpadEnter => "CR",
        KeyCode::Tab => "Tab",
        KeyCode::Space => "Space",
        KeyCode::Backspace => "BS",
        KeyCode::Escape => "Esc",
        KeyCode::Delete => "Del",
        KeyCode::Insert => "Insert",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "PageUp",
        KeyCode::PageDown => "PageDown",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        // Numpad keys
        KeyCode::Numpad0 => "k0",
        KeyCode::Numpad1 => "k1",
        KeyCode::Numpad2 => "k2",
        KeyCode::Numpad3 => "k3",
        KeyCode::Numpad4 => "k4",
        KeyCode::Numpad5 => "k5",
        KeyCode::Numpad6 => "k6",
        KeyCode::Numpad7 => "k7",
        KeyCode::Numpad8 => "k8",
        KeyCode::Numpad9 => "k9",
        KeyCode::NumpadAdd => "kPlus",
        KeyCode::NumpadSubtract => "kMinus",
        KeyCode::NumpadMultiply => "kMultiply",
        KeyCode::NumpadDivide => "kDivide",
        KeyCode::NumpadDecimal => "kPoint",
        _ => return None,
    };
    Some(s.to_string())
}

fn format_with_modifiers(key: &str, modifiers: &Modifiers, is_special: bool) -> Option<String> {
    let has_modifiers = modifiers.ctrl || modifiers.alt || modifiers.logo;
    let shift_relevant = modifiers.shift && (is_special || has_modifiers);

    if !has_modifiers && !shift_relevant {
        if is_special {
            return Some(format!("<{}>", key));
        }
        // Handle special characters that need escaping
        return Some(escape_literal(key));
    }

    let mut prefix = String::new();
    if modifiers.logo {
        prefix.push_str("D-");
    }
    if modifiers.ctrl {
        prefix.push_str("C-");
    }
    if modifiers.alt {
        prefix.push_str("M-");
    }
    if shift_relevant {
        prefix.push_str("S-");
    }

    Some(format!("<{}{}>", prefix, key))
}

fn escape_literal(key: &str) -> String {
    match key {
        "<" => "<lt>".to_string(),
        "\\" => "<Bslash>".to_string(),
        "|" => "<Bar>".to_string(),
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_mods() -> Modifiers {
        Modifiers::default()
    }

    fn with_ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            ..Default::default()
        }
    }

    fn with_alt() -> Modifiers {
        Modifiers {
            alt: true,
            ..Default::default()
        }
    }

    fn with_shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Default::default()
        }
    }

    fn with_logo() -> Modifiers {
        Modifiers {
            logo: true,
            ..Default::default()
        }
    }

    fn with_ctrl_shift() -> Modifiers {
        Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_named_key_enter() {
        assert_eq!(named_key_to_str(NamedKey::Enter), Some("CR".to_string()));
    }

    #[test]
    fn test_named_key_escape() {
        assert_eq!(named_key_to_str(NamedKey::Escape), Some("Esc".to_string()));
    }

    #[test]
    fn test_named_key_tab() {
        assert_eq!(named_key_to_str(NamedKey::Tab), Some("Tab".to_string()));
    }

    #[test]
    fn test_named_key_backspace() {
        assert_eq!(
            named_key_to_str(NamedKey::Backspace),
            Some("BS".to_string())
        );
    }

    #[test]
    fn test_named_key_space() {
        assert_eq!(named_key_to_str(NamedKey::Space), Some("Space".to_string()));
    }

    #[test]
    fn test_named_key_delete() {
        assert_eq!(named_key_to_str(NamedKey::Delete), Some("Del".to_string()));
    }

    #[test]
    fn test_named_key_arrows() {
        assert_eq!(named_key_to_str(NamedKey::ArrowUp), Some("Up".to_string()));
        assert_eq!(
            named_key_to_str(NamedKey::ArrowDown),
            Some("Down".to_string())
        );
        assert_eq!(
            named_key_to_str(NamedKey::ArrowLeft),
            Some("Left".to_string())
        );
        assert_eq!(
            named_key_to_str(NamedKey::ArrowRight),
            Some("Right".to_string())
        );
    }

    #[test]
    fn test_named_key_function_keys() {
        assert_eq!(named_key_to_str(NamedKey::F1), Some("F1".to_string()));
        assert_eq!(named_key_to_str(NamedKey::F12), Some("F12".to_string()));
    }

    #[test]
    fn test_named_key_navigation() {
        assert_eq!(named_key_to_str(NamedKey::Home), Some("Home".to_string()));
        assert_eq!(named_key_to_str(NamedKey::End), Some("End".to_string()));
        assert_eq!(
            named_key_to_str(NamedKey::PageUp),
            Some("PageUp".to_string())
        );
        assert_eq!(
            named_key_to_str(NamedKey::PageDown),
            Some("PageDown".to_string())
        );
        assert_eq!(
            named_key_to_str(NamedKey::Insert),
            Some("Insert".to_string())
        );
    }

    #[test]
    fn test_modifier_keys_return_none() {
        assert_eq!(named_key_to_str(NamedKey::Shift), None);
        assert_eq!(named_key_to_str(NamedKey::Control), None);
        assert_eq!(named_key_to_str(NamedKey::Alt), None);
        assert_eq!(named_key_to_str(NamedKey::Super), None);
    }

    #[test]
    fn test_lock_keys_return_none() {
        assert_eq!(named_key_to_str(NamedKey::NumLock), None);
        assert_eq!(named_key_to_str(NamedKey::CapsLock), None);
        assert_eq!(named_key_to_str(NamedKey::ScrollLock), None);
    }

    #[test]
    fn test_physical_keycode_numpad() {
        assert_eq!(
            physical_keycode_to_str(KeyCode::Numpad0),
            Some("k0".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::Numpad9),
            Some("k9".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::NumpadAdd),
            Some("kPlus".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::NumpadSubtract),
            Some("kMinus".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::NumpadMultiply),
            Some("kMultiply".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::NumpadDivide),
            Some("kDivide".to_string())
        );
        assert_eq!(
            physical_keycode_to_str(KeyCode::NumpadDecimal),
            Some("kPoint".to_string())
        );
    }

    #[test]
    fn test_format_regular_character() {
        assert_eq!(
            format_with_modifiers("a", &no_mods(), false),
            Some("a".to_string())
        );
    }

    #[test]
    fn test_format_special_key_no_modifiers() {
        assert_eq!(
            format_with_modifiers("CR", &no_mods(), true),
            Some("<CR>".to_string())
        );
    }

    #[test]
    fn test_format_with_ctrl() {
        assert_eq!(
            format_with_modifiers("a", &with_ctrl(), false),
            Some("<C-a>".to_string())
        );
    }

    #[test]
    fn test_format_with_alt() {
        assert_eq!(
            format_with_modifiers("a", &with_alt(), false),
            Some("<M-a>".to_string())
        );
    }

    #[test]
    fn test_format_with_logo() {
        assert_eq!(
            format_with_modifiers("a", &with_logo(), false),
            Some("<D-a>".to_string())
        );
    }

    #[test]
    fn test_format_special_key_with_shift() {
        assert_eq!(
            format_with_modifiers("Tab", &with_shift(), true),
            Some("<S-Tab>".to_string())
        );
    }

    #[test]
    fn test_format_regular_char_shift_ignored() {
        // Shift on regular chars is usually handled by the key text (a -> A)
        assert_eq!(
            format_with_modifiers("A", &with_shift(), false),
            Some("A".to_string())
        );
    }

    #[test]
    fn test_format_with_ctrl_shift() {
        assert_eq!(
            format_with_modifiers("a", &with_ctrl_shift(), false),
            Some("<C-S-a>".to_string())
        );
    }

    #[test]
    fn test_format_multiple_modifiers() {
        let mods = Modifiers {
            ctrl: true,
            alt: true,
            shift: true,
            logo: true,
        };
        assert_eq!(
            format_with_modifiers("a", &mods, false),
            Some("<D-C-M-S-a>".to_string())
        );
    }

    #[test]
    fn test_escape_less_than() {
        assert_eq!(escape_literal("<"), "<lt>".to_string());
    }

    #[test]
    fn test_escape_backslash() {
        assert_eq!(escape_literal("\\"), "<Bslash>".to_string());
    }

    #[test]
    fn test_escape_bar() {
        assert_eq!(escape_literal("|"), "<Bar>".to_string());
    }

    #[test]
    fn test_escape_regular_char() {
        assert_eq!(escape_literal("a"), "a".to_string());
    }

    #[test]
    fn test_modifiers_from_state() {
        let state = ModifiersState::CONTROL | ModifiersState::ALT;
        let mods = Modifiers::from(state);
        assert!(mods.ctrl);
        assert!(mods.alt);
        assert!(!mods.shift);
        assert!(!mods.logo);
    }

    #[test]
    fn test_modifiers_default() {
        let mods = Modifiers::default();
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.shift);
        assert!(!mods.logo);
    }
}
