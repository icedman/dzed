use crate::actions::{Action, Mode, SelectInKind};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('-').collect();
        let mut modifiers = KeyModifiers::empty();
        let mut code = None;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // The last part is the KeyCode
                let part_lower = part.to_lowercase();
                code = Some(match part_lower.as_str() {
                    "esc" | "escape" => KeyCode::Esc,
                    "enter" | "return" => KeyCode::Enter,
                    "tab" => KeyCode::Tab,
                    "backspace" => KeyCode::Backspace,
                    "delete" | "del" => KeyCode::Delete,
                    "left" => KeyCode::Left,
                    "right" => KeyCode::Right,
                    "up" => KeyCode::Up,
                    "down" => KeyCode::Down,
                    "pageup" | "pgup" => KeyCode::PageUp,
                    "pagedown" | "pgdn" => KeyCode::PageDown,
                    "home" => KeyCode::Home,
                    "end" => KeyCode::End,
                    _ if part.chars().count() == 1 => {
                        let ch = part.chars().next().unwrap();
                        KeyCode::Char(ch)
                    }
                    _ => return Err(format!("Unknown key code: {}", part)),
                });
            } else {
                // Modifiers
                let part_lower = part.to_lowercase();
                match part_lower.as_str() {
                    "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                    "alt" | "option" => modifiers.insert(KeyModifiers::ALT),
                    "shift" => modifiers.insert(KeyModifiers::SHIFT),
                    _ => return Err(format!("Unknown modifier: {}", part)),
                }
            }
        }

        if let Some(mut code) = code {
            // Normalize Char key codes under Shift modifier to uppercase
            if modifiers.contains(KeyModifiers::SHIFT) {
                if let KeyCode::Char(c) = code {
                    if c.is_ascii_lowercase() {
                        code = KeyCode::Char(c.to_ascii_uppercase());
                    }
                }
            }
            Ok(Self { code, modifiers })
        } else {
            Err("Empty key binding".to_string())
        }
    }
}

impl From<&KeyEvent> for KeyCombo {
    fn from(event: &KeyEvent) -> Self {
        let mut code = event.code;
        let mut modifiers = event.modifiers;
        // Normalize
        if modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = code {
                if c.is_ascii_lowercase() {
                    code = KeyCode::Char(c.to_ascii_uppercase());
                }
            }
        }
        Self { code, modifiers }
    }
}

pub struct Keymap {
    pub normal_actions: HashMap<KeyCombo, Action>,
    pub insert_actions: HashMap<KeyCombo, Action>,
    pub pending_commands: HashMap<String, Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut normal_actions = HashMap::new();
        let mut insert_actions = HashMap::new();
        let mut pending_commands = HashMap::new();

        // 1. Normal-mode commands
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('i'), KeyModifiers::empty()),
            Action::SetInsertMode,
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('I'), KeyModifiers::empty()),
            Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveToStartOfLine { select: false }),
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('a'), KeyModifiers::empty()),
            Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveRight {
                    select: false,
                    count: 1,
                }),
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('A'), KeyModifiers::empty()),
            Action::SetInsertModeMotion {
                motion: Box::new(Action::MoveToEndOfLine { select: false }),
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('V'), KeyModifiers::empty()),
            Action::SetVisualLineMode,
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            Action::SetVisualBlockMode,
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('v'), KeyModifiers::empty()),
            Action::SetVisualMode,
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char(':'), KeyModifiers::empty()),
            Action::SetCommandMode {
                search: false,
                pattern: false,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('/'), KeyModifiers::empty()),
            Action::SetCommandMode {
                search: true,
                pattern: false,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('?'), KeyModifiers::empty()),
            Action::SetCommandMode {
                search: true,
                pattern: true,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            Action::Redo { count: 1 },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('u'), KeyModifiers::empty()),
            Action::Undo { count: 1 },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('h'), KeyModifiers::empty()),
            Action::MoveLeft {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('l'), KeyModifiers::empty()),
            Action::MoveRight {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('k'), KeyModifiers::empty()),
            Action::MoveUp {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('j'), KeyModifiers::empty()),
            Action::MoveDown {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('n'), KeyModifiers::empty()),
            Action::MoveToNextMatch {
                search: String::new(),
                pattern: false,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('N'), KeyModifiers::empty()),
            Action::MoveToPreviousMatch {
                search: String::new(),
                pattern: false,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Delete, KeyModifiers::empty()),
            Action::DeleteText { count: 1 },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Backspace, KeyModifiers::empty()),
            Action::MoveLeft {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Left, KeyModifiers::SHIFT),
            Action::MoveToPreviousWord {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Right, KeyModifiers::SHIFT),
            Action::MoveToNextWord {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Action::SelectIn {
                kind: SelectInKind::Word,
            },
        );

        // 2. Motions
        normal_actions.insert(
            KeyCombo::new(KeyCode::Left, KeyModifiers::empty()),
            Action::MoveLeft {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Right, KeyModifiers::empty()),
            Action::MoveRight {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Up, KeyModifiers::empty()),
            Action::MoveUp {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Down, KeyModifiers::empty()),
            Action::MoveDown {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::PageUp, KeyModifiers::empty()),
            Action::MoveUp {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::PageDown, KeyModifiers::empty()),
            Action::MoveDown {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Home, KeyModifiers::empty()),
            Action::MoveToStartOfLine { select: false },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::End, KeyModifiers::empty()),
            Action::MoveToEndOfLine { select: false },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('0'), KeyModifiers::empty()),
            Action::MoveToStartOfLine { select: false },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('$'), KeyModifiers::empty()),
            Action::MoveToEndOfLine { select: false },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('^'), KeyModifiers::empty()),
            Action::MoveToStartOfLineNonSpace { select: false },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('{'), KeyModifiers::empty()),
            Action::MoveToPreviousParagraph {
                select: false,
                count: 1,
            },
        );
        normal_actions.insert(
            KeyCombo::new(KeyCode::Char('}'), KeyModifiers::empty()),
            Action::MoveToNextParagraph {
                select: false,
                count: 1,
            },
        );

        // 3. Insert-mode keybindings
        insert_actions.insert(
            KeyCombo::new(KeyCode::Enter, KeyModifiers::empty()),
            Action::InsertNewLine,
        );
        insert_actions.insert(
            KeyCombo::new(KeyCode::Tab, KeyModifiers::empty()),
            Action::InsertTab,
        );
        insert_actions.insert(
            KeyCombo::new(KeyCode::Delete, KeyModifiers::empty()),
            Action::Delete { count: 1 },
        );
        insert_actions.insert(
            KeyCombo::new(KeyCode::Backspace, KeyModifiers::empty()),
            Action::Backspace,
        );

        // 4. Pending sequence commands
        pending_commands.insert(
            "iw".to_string(),
            Action::SelectIn {
                kind: SelectInKind::Word,
            },
        );
        pending_commands.insert(
            "aw".to_string(),
            Action::SelectAround {
                kind: SelectInKind::Word,
            },
        );
        pending_commands.insert(
            "gg".to_string(),
            Action::MoveToStartOfDocument { select: false },
        );
        pending_commands.insert(
            "G".to_string(),
            Action::MoveToEndOfDocument { select: false },
        );
        pending_commands.insert("dd".to_string(), Action::DeleteCurrentLine { count: 1 });
        pending_commands.insert(
            "dw".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextWord {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "db".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousWord {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "de".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextWordEnd {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "dge".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousWordEnd {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "dj".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveDown {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "dk".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveUp {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "dh".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveLeft {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "dl".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveRight {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "d0".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfLine { select: true }),
            },
        );
        pending_commands.insert(
            "d$".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToEndOfLine { select: true }),
            },
        );
        pending_commands.insert(
            "d^".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfLineNonSpace { select: true }),
            },
        );
        pending_commands.insert(
            "d{".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousParagraph {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "d}".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextParagraph {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "D".to_string(),
            Action::DeleteMotion {
                count: 1,
                motion: Box::new(Action::MoveToEndOfLine { select: true }),
            },
        );
        pending_commands.insert("cc".to_string(), Action::ChangeCurrentLine { count: 1 });
        pending_commands.insert(
            "cw".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextWord {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "cb".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousWord {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "ce".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextWordEnd {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "cge".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousWordEnd {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "cj".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveDown {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "ck".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveUp {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "ch".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveLeft {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "cl".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveRight {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "c0".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfLine { select: true }),
            },
        );
        pending_commands.insert(
            "c$".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToEndOfLine { select: true }),
            },
        );
        pending_commands.insert(
            "c^".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfLineNonSpace { select: true }),
            },
        );
        pending_commands.insert(
            "c{".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToPreviousParagraph {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert(
            "c}".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToNextParagraph {
                    select: true,
                    count: 1,
                }),
            },
        );
        pending_commands.insert("c".to_string(), Action::Change {});
        pending_commands.insert(
            "C".to_string(),
            Action::ChangeMotion {
                count: 1,
                motion: Box::new(Action::MoveToEndOfLine { select: true }),
            },
        );
        pending_commands.insert(
            "o".to_string(),
            Action::InsertNewLineMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfNextLine { select: false }),
            },
        );
        pending_commands.insert(
            "O".to_string(),
            Action::InsertNewLineMotion {
                count: 1,
                motion: Box::new(Action::MoveToStartOfLine { select: false }),
            },
        );
        pending_commands.insert("x".to_string(), Action::Delete { count: 1 });
        pending_commands.insert(
            "b".to_string(),
            Action::MoveToPreviousWord {
                select: false,
                count: 1,
            },
        );
        pending_commands.insert(
            "w".to_string(),
            Action::MoveToNextWord {
                select: false,
                count: 1,
            },
        );
        pending_commands.insert(
            "e".to_string(),
            Action::MoveToNextWordEnd {
                select: false,
                count: 1,
            },
        );
        pending_commands.insert(
            "ge".to_string(),
            Action::MoveToPreviousWordEnd {
                select: false,
                count: 1,
            },
        );

        Self {
            normal_actions,
            insert_actions,
            pending_commands,
        }
    }
}

impl Keymap {
    pub fn get_normal_action(&self, combo: &KeyCombo) -> Option<Action> {
        if let Some(action) = self.normal_actions.get(combo) {
            return Some(action.clone());
        }
        if !combo.modifiers.is_empty() {
            let fallback_combo = KeyCombo::new(combo.code, KeyModifiers::empty());
            if let Some(action) = self.normal_actions.get(&fallback_combo) {
                return Some(action.clone());
            }
        }
        None
    }

    pub fn get_insert_action(&self, combo: &KeyCombo) -> Option<Action> {
        if let Some(action) = self.insert_actions.get(combo) {
            return Some(action.clone());
        }
        if !combo.modifiers.is_empty() {
            let fallback_combo = KeyCombo::new(combo.code, KeyModifiers::empty());
            if let Some(action) = self.insert_actions.get(&fallback_combo) {
                return Some(action.clone());
            }
        }
        None
    }

    pub fn get_pending_action(&self, pending: &str) -> Option<Action> {
        self.pending_commands.get(pending).cloned()
    }

    pub fn bind_normal(&mut self, keys: &str, action: Action) -> Result<(), String> {
        let combo = KeyCombo::parse(keys)?;
        self.normal_actions.insert(combo, action);
        Ok(())
    }

    pub fn bind_insert(&mut self, keys: &str, action: Action) -> Result<(), String> {
        let combo = KeyCombo::parse(keys)?;
        self.insert_actions.insert(combo, action);
        Ok(())
    }

    pub fn bind_pending(&mut self, sequence: &str, action: Action) {
        self.pending_commands.insert(sequence.to_string(), action);
    }
}
