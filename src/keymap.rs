use crate::actions::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

/// Represents a single physical keypress with modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Parses single key patterns like "C-f", "M-x", "Esc", "a", or "S-Up".
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

/// Allows sequence patterns to specify exact keys or dynamic wildcards like `{c}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyPattern {
    Exact(KeyCombo),
    AnyChar,
}

impl KeyPattern {
    pub fn matches(&self, combo: &KeyCombo) -> bool {
        match self {
            KeyPattern::Exact(target) => target.matches(combo),
            KeyPattern::AnyChar => matches!(combo.code, KeyCode::Char(_)),
        }
    }
}

/// Sequence of key patterns stored in a keymap binding.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KeyComboSequence {
    pub items: Vec<KeyPattern>,
}

impl KeyComboSequence {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn new_from_combos(combos: Vec<KeyCombo>) -> Self {
        Self {
            items: combos.into_iter().map(KeyPattern::Exact).collect(),
        }
    }

    /// Parses key sequence strings correctly.
    /// Handles single modifiers ("C-f"), named keys ("Left"), wildcards ("f{c}"), or plain character sequences ("gg", "dd").
    pub fn parse_sequence(s: &str) -> Result<Self, String> {
        let mut items = Vec::new();

        if s == "{c}" {
            items.push(KeyPattern::AnyChar);
        } else if s.ends_with("{c}") {
            let prefix = &s[..s.len() - 3];
            let prefix_seq = Self::parse_sequence(prefix)?;
            items.extend(prefix_seq.items);
            items.push(KeyPattern::AnyChar);
        } else if s.contains('-') || KeyCombo::parse(s).is_ok() {
            let combo = KeyCombo::parse(s)?;
            items.push(KeyPattern::Exact(combo));
        } else {
            for ch in s.chars() {
                let combo = KeyCombo::parse(&ch.to_string())?;
                items.push(KeyPattern::Exact(combo));
            }
        }

        Ok(Self { items })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Deref for KeyComboSequence {
    type Target = [KeyPattern];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

/// Represents the active input state buffer for incoming user keystrokes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBuffer {
    pub items: Vec<KeyCombo>,
}

impl KeyBuffer {
    pub fn new() -> Self {
        Self { items: Vec::new() }
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

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn truncate_items(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let new_len = self.items.len().saturating_sub(n);
        self.items.truncate(new_len);
    }

    /// Evaluates if buffer's trailing items match a target pattern sequence.
    pub fn ends_with_pattern(&self, target: &KeyComboSequence) -> bool {
        if target.is_empty() || target.len() > self.items.len() {
            return false;
        }

        self.items
            .iter()
            .rev()
            .zip(target.items.iter().rev())
            .all(|(combo, pattern)| pattern.matches(combo))
    }

    /// Extracts numerical counts entered before the action sequence in correct left-to-right digit order.
    pub fn pop_trailing_digits(&mut self) -> u32 {
        let mut digit_chars = Vec::new();

        while let Some(combo) = self.items.last() {
            if let KeyCode::Char(c) = combo.code {
                if c.is_ascii_digit() && combo.modifiers.is_empty() {
                    digit_chars.push(c);
                    self.items.pop();
                    continue;
                }
            }
            break;
        }

        if digit_chars.is_empty() {
            return 1;
        }

        digit_chars.reverse();
        digit_chars
            .into_iter()
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(1)
    }
}

impl fmt::Display for KeyBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = String::new();
        for item in &self.items {
            res.push_str(&item.to_string());
        }
        write!(f, "{}", res)
    }
}

// Ergonomic traits for binding sequences.
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
        let mut items = Vec::new();
        for s in self {
            let seq = KeyComboSequence::parse_sequence(s)?;
            items.extend(seq.items);
        }
        Ok(KeyComboSequence { items })
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
        op_actions
            .bind("d", Action::Delete { count: 1 })
            .expect("Valid binding");
        op_actions
            .bind("c", Action::Change { count: 1 })
            .expect("Valid binding");
        op_actions
            .bind("y", Action::Yank { count: 1 })
            .expect("Valid binding");

        // Motions
        motion_actions
            .bind(
                "w",
                Action::MoveToWord {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "e",
                Action::MoveToWordEnd {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "b",
                Action::MoveToPreviousWord {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "ge",
                Action::MoveToPreviousWordEnd {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "W",
                Action::MoveToBigWord {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "B",
                Action::MoveToPreviousBigWord {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "E",
                Action::MoveToBigWordEnd {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "h",
                Action::MoveLeft {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "l",
                Action::MoveRight {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "k",
                Action::MoveUp {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "j",
                Action::MoveDown {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");

        motion_actions
            .bind(
                ["Left"],
                Action::MoveLeft {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                ["Right"],
                Action::MoveRight {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                ["Up"],
                Action::MoveUp {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                ["Down"],
                Action::MoveDown {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");

        motion_actions
            .bind(
                "gg",
                Action::MoveToStartOfDocument {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "G",
                Action::MoveToEndOfDocument {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "0",
                Action::MoveToStartOfLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "^",
                Action::MoveToStartOfLineNonSpace {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "$",
                Action::MoveToEndOfLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        // motion_actions.bind("-", Action::MoveToStartOfPreviousLine { count: 1, select: false }).expect("Valid binding");
        motion_actions
            .bind(
                "+",
                Action::MoveToStartOfNextLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        // motion_actions.bind("g-", Action::MoveToEndOfPreviousLine { count: 1, select: false }).expect("Valid binding");
        motion_actions
            .bind(
                "g+",
                Action::MoveToEndOfNextLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");

        motion_actions
            .bind(
                "H",
                Action::MoveToScreenTop {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "M",
                Action::MoveToScreenMiddle {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "L",
                Action::MoveToScreenBottom {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "{",
                Action::MoveToPreviousParagraph {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "}",
                Action::MoveToNextParagraph {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "(",
                Action::MoveToPreviousSentence {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                ")",
                Action::MoveToNextSentence {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");

        motion_actions
            .bind(
                "f{c}",
                Action::MoveToNextCharacter {
                    count: 1,
                    select: false,
                    ch: '?',
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "F{c}",
                Action::MoveToPreviousCharacter {
                    count: 1,
                    select: false,
                    ch: '?',
                },
            )
            .expect("Valid binding");

        motion_actions
            .bind("C-f", Action::ScrollForward { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("C-b", Action::ScrollBackward { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("C-d", Action::ScrollHalfPageDown { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("C-u", Action::ScrollHalfPageUp { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("C-e", Action::ScrollLineDown { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("C-y", Action::ScrollLineUp { count: 1 })
            .expect("Valid binding");

        motion_actions
            .bind("|", Action::MoveToColumn { count: 1 })
            .expect("Valid binding");

        motion_actions
            .bind("/", Action::SearchForward { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("?", Action::SearchBackward { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("n", Action::SearchNext { count: 1 })
            .expect("Valid binding");
        motion_actions
            .bind("N", Action::SearchPrevious { count: 1 })
            .expect("Valid binding");

        motion_actions
            .bind(
                ["End"],
                Action::MoveToEndOfLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                ["Home"],
                Action::MoveToStartOfLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");

        // Normal Mode
        normal_actions
            .bind("dd", Action::DeleteLine { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("cc", Action::ChangeLine { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("yy", Action::YankLine { count: 1 })
            .expect("Valid binding");

        normal_actions
            .bind("x", Action::DeleteChar { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("X", Action::DeleteCharBefore { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("p", Action::Put { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("P", Action::PutBefore { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("J", Action::JoinLines { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("u", Action::Undo { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("C-r", Action::Redo { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind(".", Action::Repeat { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind(">", Action::Indent { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("<", Action::Outdent { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind("~", Action::ChangeCase { count: 1 })
            .expect("Valid binding");

        normal_actions
            .bind(["Delete"], Action::DeleteChar { count: 1 })
            .expect("Valid binding");
        normal_actions
            .bind(
                ["Backspace"],
                Action::MoveLeft {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        normal_actions
            .bind(["Esc"], Action::Clear)
            .expect("Valid binding");

        // Mode Change
        mode_actions
            .bind("i", Action::SetToInsert)
            .expect("Valid binding");
        mode_actions
            .bind("v", Action::SetToVisual)
            .expect("Valid binding");
        mode_actions
            .bind("V", Action::SetToVisualLine)
            .expect("Valid binding");
        mode_actions
            .bind("C-v", Action::SetToVisualBlock)
            .expect("Valid binding");
        mode_actions
            .bind(":", Action::SetToCommand)
            .expect("Valid binding");

        // Insert Mode
        insert_actions
            .bind(
                ["Left"],
                Action::MoveLeft {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        insert_actions
            .bind(
                ["Right"],
                Action::MoveRight {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        insert_actions
            .bind(
                ["Up"],
                Action::MoveUp {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        insert_actions
            .bind(
                ["Down"],
                Action::MoveDown {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        insert_actions
            .bind(
                ["PageUp"],
                Action::MovePageUp {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        insert_actions
            .bind(["Esc"], Action::Clear)
            .expect("Valid binding");
        insert_actions
            .bind(["Enter"], Action::InsertNewLine { count: 1 })
            .expect("Valid binding");
        insert_actions
            .bind(["Tab"], Action::InsertTab)
            .expect("Valid binding");
        insert_actions
            .bind(["Backspace"], Action::DeleteCharBefore { count: 1 })
            .expect("Valid binding");
        insert_actions
            .bind(["Delete"], Action::DeleteChar { count: 1 })
            .expect("Valid binding");

        // Visual Mode
        visual_actions
            .bind(["Esc"], Action::Clear)
            .expect("Valid binding");

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

pub fn peek_action(seq: &KeyBuffer, map: &HashMap<KeyComboSequence, Action>) -> Action {
    let mut s = seq.clone();
    resolve_action(&mut s, map)
}

pub fn peek_count(seq: &KeyBuffer) -> u32 {
    let mut s = seq.clone();
    resolve_count(&mut s)
}

pub fn resolve_count(seq: &mut KeyBuffer) -> u32 {
    seq.pop_trailing_digits()
}

/// Resolves an action from the KeyBuffer using longest-prefix pattern matching.
pub fn resolve_action(
    seq: &mut KeyBuffer,
    map: &HashMap<KeyComboSequence, Action>,
) -> Action {
    let mut matched: Option<(KeyComboSequence, Action)> = None;

    // Find the longest sequence pattern that matches the buffer tail.
    for (key, action) in map {
        if seq.ends_with_pattern(key) {
            // DISAMBIGUATE '0' MOTION VS '0' IN A COUNT (e.g., '10w'):
            // If the matched key is '0' alone, but it is preceded by a digit 1-9,
            // then '0' is part of a count (like 10), NOT the standalone '0' motion.
            if key.len() == 1 {
                if let Some(KeyPattern::Exact(combo)) = key.items.first() {
                    if combo.code == KeyCode::Char('0') && combo.modifiers.is_empty() {
                        let match_idx = seq.items.len().saturating_sub(1);
                        if match_idx > 0 {
                            if let KeyCode::Char(prev_ch) = seq.items[match_idx - 1].code {
                                if prev_ch.is_ascii_digit() && prev_ch != '0' {
                                    continue; // Skip '0' motion match! It's part of a count.
                                }
                            }
                        }
                    }
                }
            }

            if let Some((ref best_key, _)) = matched {
                if key.len() > best_key.len() {
                    matched = Some((key.clone(), action.clone()));
                }
            } else {
                matched = Some((key.clone(), action.clone()));
            }
        }
    }

    if let Some((key, mut action)) = matched {
        let has_wildcard = key.items.iter().any(|p| matches!(p, KeyPattern::AnyChar));
        let last_combo = seq.items.last().cloned();

        // Truncate matched pattern key items from buffer tail
        seq.truncate_items(key.len());

        // Extract count digits preceding the sequence (e.g. '10' before 'w')
        let count = resolve_count(seq);

        if has_wildcard {
            if let Some(combo) = last_combo {
                if let KeyCode::Char(ch) = combo.code {
                    action = action.with_char(ch, count);
                } else {
                    return Action::NoOp;
                }
            }
        } else {
            action = action.with_count(count);
        }

        return action;
    }

    Action::NoOp
}

pub fn resolve_op_motion_action(motion: Action, action: Action) -> Action {
    let count = action.count();
    match action {
        Action::Delete { .. } => Action::DeleteMotion {
            count,
            motion: Box::new(motion),
        },
        Action::Change { .. } => Action::ChangeMotion {
            count,
            motion: Box::new(motion),
        },
        Action::Yank { .. } => Action::YankMotion {
            count,
            motion: Box::new(motion),
        },
        _ => Action::NoOp,
    }
}
