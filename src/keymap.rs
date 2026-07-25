use crate::actions::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
                let part_lower = part.to_lowercase();
                code = Some(match part_lower.as_str() {
                    "esc" | "escape" => KeyCode::Esc,
                    "enter" | "return" => KeyCode::Enter,
                    "tab" => KeyCode::Tab,
                    "backtab" => KeyCode::BackTab,
                    "backspace" => KeyCode::Backspace,
                    "delete" | "del" => KeyCode::Delete,
                    "insert" | "ins" => KeyCode::Insert,
                    "left" => KeyCode::Left,
                    "right" => KeyCode::Right,
                    "up" => KeyCode::Up,
                    "down" => KeyCode::Down,
                    "pagup" | "pgup" => KeyCode::PageUp,
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
                let part_lower = part.to_lowercase();
                match part_lower.as_str() {
                    "c" | "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                    "m" | "alt" | "option" => modifiers.insert(KeyModifiers::ALT),
                    "s" | "shift" => modifiers.insert(KeyModifiers::SHIFT),
                    _ => return Err(format!("Unknown modifier: {}", part)),
                }
            }
        }

        if let Some(mut code) = code {
            if modifiers.contains(KeyModifiers::SHIFT) {
                if let KeyCode::Char(c) = code {
                    if c.is_ascii_alphabetic() {
                        code = KeyCode::Char(c.to_ascii_uppercase());
                        modifiers.remove(KeyModifiers::SHIFT);
                    }
                }
            }
            Ok(Self { code, modifiers })
        } else {
            Err("Empty key binding".to_string())
        }
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            s.push_str("C-");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            s.push_str("M-");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            s.push_str("S-");
        }

        match self.code {
            KeyCode::Char(c) => s.push(c),
            KeyCode::Esc => s.push_str("Esc"),
            KeyCode::Enter => s.push_str("Enter"),
            KeyCode::Backspace => s.push_str("Backspace"),
            KeyCode::Tab => s.push_str("Tab"),
            KeyCode::Up => s.push_str("Up"),
            KeyCode::Down => s.push_str("Down"),
            KeyCode::Left => s.push_str("Left"),
            KeyCode::Right => s.push_str("Right"),
            KeyCode::PageUp => s.push_str("PageUp"),
            KeyCode::PageDown => s.push_str("PageDown"),
            KeyCode::Home => s.push_str("Home"),
            KeyCode::End => s.push_str("End"),
            _ => s.push_str(&format!("{:?}", self.code)),
        }
        s
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.code == other.code && self.modifiers == other.modifiers
    }
}

impl std::fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyComboSequence {
    items: Vec<KeyCombo>,
}

impl KeyComboSequence {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_from_strings(keys: Vec<&str>) -> Self {
        let mut items = Vec::new();
        for s in keys {
            if let Ok(combo) = KeyCombo::parse(s) {
                items.push(combo);
            }
        }
        Self { items }
    }

    /// Parses sequence strings into individual combos.
    /// Handles single modifiers ("C-f") or plain key sequences ("gg", "dd", "ge").
    pub fn parse_sequence(s: &str) -> Result<Self, String> {
        let mut items = Vec::new();

        if s.contains('-') {
            let combo = KeyCombo::parse(s)?;
            items.push(combo);
        } else {
            for ch in s.chars() {
                let combo = KeyCombo::parse(&ch.to_string())?;
                items.push(combo);
            }
        }

        Ok(Self { items })
    }

    pub fn push(&mut self, item: KeyCombo) {
        self.items.push(item);
    }

    pub fn pop(&mut self) -> Option<KeyCombo> {
        self.items.pop()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn truncate_items(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let new_len = self.items.len().saturating_sub(n);
        self.items.truncate(new_len);
    }

    pub fn pop_trailing_digits(&mut self) -> u32 {
        let mut digits = String::new();

        while let Some(combo) = self.items.last() {
            if let KeyCode::Char(c) = combo.code {
                if c.is_ascii_digit() {
                    self.items.pop();
                    digits.push(c);
                    continue;
                }
            }
            break;
        }

        if digits.is_empty() {
            return 1;
        }

        digits
            .chars()
            .rev()
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(1)
    }

    pub fn ends_with_seq(&self, target: &[KeyCombo]) -> bool {
        if target.is_empty() || target.len() > self.items.len() {
            return false;
        }

        self.items
            .iter()
            .rev()
            .zip(target.iter().rev())
            .all(|(a, b)| a.matches(b))
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl Deref for KeyComboSequence {
    type Target = [KeyCombo];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl std::fmt::Display for KeyComboSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        for item in &self.items {
            res.push_str(&item.to_string());
        }
        write!(f, "{}", res)
    }
}

// Flexible binding helper trait supporting string slices or single str references
pub trait IntoKeyComboSequence {
    fn into_seq(self) -> Result<KeyComboSequence, String>;
}

impl IntoKeyComboSequence for &str {
    fn into_seq(self) -> Result<KeyComboSequence, String> {
        KeyComboSequence::parse_sequence(self)
    }
}

impl<const N: usize> IntoKeyComboSequence for [&str; N] {
    fn into_seq(self) -> Result<KeyComboSequence, String> {
        Ok(KeyComboSequence::new_from_strings(self.to_vec()))
    }
}

pub trait BindSequence {
    fn bind<K: IntoKeyComboSequence>(&mut self, keys: K, action: Action) -> Result<(), String>;
}

impl BindSequence for HashMap<KeyComboSequence, Action> {
    fn bind<K: IntoKeyComboSequence>(&mut self, keys: K, action: Action) -> Result<(), String> {
        let seq = keys.into_seq()?;
        if seq.is_empty() {
            return Err("Failed to parse key combo sequence".to_string());
        }
        self.insert(seq, action);
        Ok(())
    }
}

impl From<&KeyEvent> for KeyCombo {
    fn from(event: &KeyEvent) -> Self {
        let mut code = event.code;
        let mut modifiers = event.modifiers;

        if modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = code {
                if c.is_ascii_alphabetic() {
                    code = KeyCode::Char(c.to_ascii_uppercase());
                    modifiers.remove(KeyModifiers::SHIFT);
                }
            }
        }

        Self { code, modifiers }
    }
}

pub struct Keymap {
    pub op_actions: HashMap<KeyComboSequence, Action>,
    pub motion_actions: HashMap<KeyComboSequence, Action>,
    pub normal_actions: HashMap<KeyComboSequence, Action>,
    pub mode_actions: HashMap<KeyComboSequence, Action>,
    pub insert_actions: HashMap<KeyComboSequence, Action>,
    pub visual_actions: HashMap<KeyComboSequence, Action>,
}

impl Keymap {
    pub fn new() -> Self {
        let mut op_actions = HashMap::new();
        let mut motion_actions = HashMap::new();
        let mut normal_actions = HashMap::new();
        let mut mode_actions = HashMap::new();
        let mut insert_actions = HashMap::new();
        let mut visual_actions = HashMap::new();

        // Operators
        let _ = op_actions.bind("d", Action::Delete { count: 1 });
        let _ = op_actions.bind("c", Action::Change { count: 1 });
        let _ = op_actions.bind("y", Action::Yank { count: 1 });

        // Motions
        let _ = motion_actions.bind("w", Action::MoveToWord { count: 1, select: false });
        let _ = motion_actions.bind("e", Action::MoveToWordEnd { count: 1, select: false });
        let _ = motion_actions.bind("b", Action::MoveToPreviousWord { count: 1, select: false });
        let _ = motion_actions.bind("ge", Action::MoveToPreviousWordEnd { count: 1, select: false });
        let _ = motion_actions.bind("W", Action::MoveToBigWord { count: 1, select: false });
        let _ = motion_actions.bind("B", Action::MoveToPreviousBigWord { count: 1, select: false });
        let _ = motion_actions.bind("E", Action::MoveToBigWordEnd { count: 1, select: false });
        let _ = motion_actions.bind("h", Action::MoveLeft { count: 1, select: false });
        let _ = motion_actions.bind("l", Action::MoveRight { count: 1, select: false });
        let _ = motion_actions.bind("k", Action::MoveUp { count: 1, select: false });
        let _ = motion_actions.bind("j", Action::MoveDown { count: 1, select: false });

        let _ = motion_actions.bind(["Left"], Action::MoveLeft { count: 1, select: false });
        let _ = motion_actions.bind(["Right"], Action::MoveRight { count: 1, select: false });
        let _ = motion_actions.bind(["Up"], Action::MoveUp { count: 1, select: false });
        let _ = motion_actions.bind(["Down"], Action::MoveDown { count: 1, select: false });

        let _ = motion_actions.bind("gg", Action::MoveToStartOfDocument { count: 1, select: false });
        let _ = motion_actions.bind("G", Action::MoveToEndOfDocument { count: 1, select: false });
        let _ = motion_actions.bind("0", Action::MoveToStartOfLine { count: 1, select: false });
        let _ = motion_actions.bind("^", Action::MoveToStartOfLineNonSpace { count: 1, select: false });
        let _ = motion_actions.bind("$", Action::MoveToEndOfLine { count: 1, select: false });
        let _ = motion_actions.bind("-", Action::MoveToStartOfPreviousLine { count: 1, select: false });
        let _ = motion_actions.bind("+", Action::MoveToStartOfNextLine { count: 1, select: false });
        let _ = motion_actions.bind("g-", Action::MoveToEndOfPreviousLine { count: 1, select: false });
        let _ = motion_actions.bind("g+", Action::MoveToEndOfNextLine { count: 1, select: false });

        let _ = motion_actions.bind("H", Action::MoveToScreenTop { count: 1, select: false });
        let _ = motion_actions.bind("M", Action::MoveToScreenMiddle { count: 1, select: false });
        let _ = motion_actions.bind("L", Action::MoveToScreenBottom { count: 1, select: false });
        let _ = motion_actions.bind("{", Action::MoveToPreviousParagraph { count: 1, select: false });
        let _ = motion_actions.bind("}", Action::MoveToNextParagraph { count: 1, select: false });
        let _ = motion_actions.bind("(", Action::MoveToPreviousSentence { count: 1, select: false });
        let _ = motion_actions.bind(")", Action::MoveToNextSentence { count: 1, select: false });

        let _ = motion_actions.bind("f{c}", Action::MoveToNextCharacter { count: 1, select: false, ch: '?' });
        let _ = motion_actions.bind("F{c}", Action::MoveToPreviousCharacter { count: 1, select: false, ch: '?' });

        let _ = motion_actions.bind("C-f", Action::ScrollForward { count: 1 });
        let _ = motion_actions.bind("C-b", Action::ScrollBackward { count: 1 });
        let _ = motion_actions.bind("C-d", Action::ScrollHalfPageDown { count: 1 });
        let _ = motion_actions.bind("C-u", Action::ScrollHalfPageUp { count: 1 });
        let _ = motion_actions.bind("C-e", Action::ScrollLineDown { count: 1 });
        let _ = motion_actions.bind("C-y", Action::ScrollLineUp { count: 1 });

        let _ = motion_actions.bind("|", Action::MoveToColumn { count: 1 });

        let _ = motion_actions.bind("/", Action::SearchForward { count: 1 });
        let _ = motion_actions.bind("?", Action::SearchBackward { count: 1 });
        let _ = motion_actions.bind("n", Action::SearchNext { count: 1 });
        let _ = motion_actions.bind("N", Action::SearchPrevious { count: 1 });

        let _ = motion_actions.bind(["End"], Action::MoveToEndOfLine { count: 1, select: false });
        let _ = motion_actions.bind(["Home"], Action::MoveToStartOfLine { count: 1, select: false });

        // Normal Mode
        let _ = normal_actions.bind("dd", Action::DeleteLine { count: 1 });
        let _ = normal_actions.bind("cc", Action::ChangeLine { count: 1 });
        let _ = normal_actions.bind("yy", Action::YankLine { count: 1 });

        let _ = normal_actions.bind("x", Action::DeleteChar { count: 1 });
        let _ = normal_actions.bind("X", Action::DeleteCharBefore { count: 1 });
        let _ = normal_actions.bind("p", Action::Put { count: 1 });
        let _ = normal_actions.bind("P", Action::PutBefore { count: 1 });
        let _ = normal_actions.bind("J", Action::JoinLines { count: 1 });
        let _ = normal_actions.bind("u", Action::Undo { count: 1 });
        let _ = normal_actions.bind("C-r", Action::Redo { count: 1 });
        let _ = normal_actions.bind(".", Action::Repeat { count: 1 });
        let _ = normal_actions.bind(">", Action::Indent { count: 1 });
        let _ = normal_actions.bind("<", Action::Outdent { count: 1 });
        let _ = normal_actions.bind("~", Action::ChangeCase { count: 1 });

        let _ = normal_actions.bind(["Delete"], Action::DeleteChar { count: 1 });
        let _ = normal_actions.bind(["Backspace"], Action::MoveLeft { count: 1, select: false });
        let _ = normal_actions.bind(["Esc"], Action::Clear);

        // Mode Change
        let _ = mode_actions.bind("i", Action::SetToInsert);
        let _ = mode_actions.bind("v", Action::SetToVisual);
        let _ = mode_actions.bind("V", Action::SetToVisualLine);
        let _ = mode_actions.bind("C-v", Action::SetToVisualBlock);
        let _ = mode_actions.bind(":", Action::SetToCommand);

        // Insert Mode
        let _ = insert_actions.bind(["Left"], Action::MoveLeft { count: 1, select: false });
        let _ = insert_actions.bind(["Right"], Action::MoveRight { count: 1, select: false });
        let _ = insert_actions.bind(["Up"], Action::MoveUp { count: 1, select: false });
        let _ = insert_actions.bind(["Down"], Action::MoveDown { count: 1, select: false });
        let _ = insert_actions.bind(["PageUp"], Action::MovePageUp { count: 1, select: false });
        let _ = insert_actions.bind(["Esc"], Action::Clear);
        let _ = insert_actions.bind(["Enter"], Action::InsertNewLine { count: 1 });
        let _ = insert_actions.bind(["Tab"], Action::InsertTab);
        let _ = insert_actions.bind(["Backspace"], Action::DeleteCharBefore { count: 1 });
        let _ = insert_actions.bind(["Delete"], Action::DeleteChar { count: 1 });
        let _ = insert_actions.bind("{c}", Action::InsertText("".to_string()));

        // Visual Mode
        let _ = visual_actions.bind(["Esc"], Action::Clear);

        Self {
            op_actions,
            motion_actions,
            mode_actions,
            normal_actions,
            insert_actions,
            visual_actions,
        }
    }
}