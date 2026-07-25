use crate::actions::{Action, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::ops::Deref;
use smallvec::SmallVec;

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

    /// Parses single key strings like "C-f", "M-x", "Esc", "a", or "S-Up".
    pub fn parse(s: &str) -> Result<Self, String> {
        if s == "-" {
            return Ok(Self {
                code: KeyCode::Char('-'),
                modifiers: KeyModifiers::empty(),
            });
        }

        let parts: Vec<&str> = if s.ends_with("--") {
            let mut p: Vec<&str> = s[..s.len() - 1].split('-').collect();
            if let Some(last) = p.last_mut() {
                if last.is_empty() {
                    *last = "-";
                }
            }
            p
        } else {
            s.split('-').collect()
        };

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
    pub items: SmallVec<[KeyPattern; 4]>,
}

impl KeyComboSequence {
    pub fn new() -> Self {
        Self { items: SmallVec::new() }
    }

    pub fn parse_sequence(s: &str) -> Result<Self, String> {
        let mut items = SmallVec::new();

        if s == "{c}" {
            items.push(KeyPattern::AnyChar);
        } else if s.ends_with("{c}") {
            let prefix = &s[..s.len() - 3];
            let prefix_seq = Self::parse_sequence(prefix)?;
            items.extend(prefix_seq.items);
            items.push(KeyPattern::AnyChar);
        } else if s.starts_with('-') || KeyCombo::parse(s).is_ok() {
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
        let mut items = SmallVec::new();
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
        motion_actions
            .bind(
                "-",
                Action::MoveToStartOfPreviousLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "+",
                Action::MoveToStartOfNextLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
        motion_actions
            .bind(
                "g-",
                Action::MoveToEndOfPreviousLine {
                    count: 1,
                    select: false,
                },
            )
            .expect("Valid binding");
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

/// Incremental State Machine for parsing key combinations from the front.
pub struct InputStateMachine {
    pub mode: Mode,
    pub count_buffer: String,
    pub key_sequence: SmallVec<[KeyCombo; 4]>,
    pub pending_op: Option<Action>,
    pub pending_op_sequence: SmallVec<[KeyCombo; 4]>,
    pub op_count: u32,
}

impl InputStateMachine {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            count_buffer: String::new(),
            key_sequence: SmallVec::new(),
            pending_op: None,
            pending_op_sequence: SmallVec::new(),
            op_count: 1,
        }
    }

    pub fn clear(&mut self) {
        self.count_buffer.clear();
        self.key_sequence.clear();
        self.pending_op = None;
        self.pending_op_sequence.clear();
        self.op_count = 1;
    }

    pub fn process_key(&mut self, combo: KeyCombo, keymap: &Keymap) -> Action {
        // 1. Insert & Command Mode Input
        if self.mode == Mode::Insert || self.mode == Mode::Command {
            return self.process_insert_mode(combo, keymap);
        }

        // 2. Count Digits vs Standalone '0' Motion
        if combo.modifiers.is_empty() {
            if let KeyCode::Char(c) = combo.code {
                if c.is_ascii_digit() && (c != '0' || !self.count_buffer.is_empty()) {
                    self.count_buffer.push(c);
                    return Action::NoOp;
                }
            }
        }

        // 3. Sequence Matching
        self.key_sequence.push(combo);

        // Try combined sequence (pending op seq + current key seq) first if there is a pending op
        let mut resolved = MatchResult::NoMatch;
        let mut used_combined = false;
        if self.pending_op.is_some() && !self.pending_op_sequence.is_empty() {
            let mut combined: SmallVec<[KeyCombo; 8]> = SmallVec::new();
            combined.extend(self.pending_op_sequence.iter().cloned());
            combined.extend(self.key_sequence.iter().cloned());
            let res = self.try_resolve_sequence(&combined, keymap);
            if !matches!(res, MatchResult::NoMatch) {
                resolved = res;
                used_combined = true;
            }
        }

        if matches!(resolved, MatchResult::NoMatch) {
            resolved = self.try_resolve(keymap);
        }

        match resolved {
            MatchResult::Complete(mut action) => {
                let motion_count = self.take_count();

                // Update mode if action is a mode transition
                match action {
                    Action::SetToInsert => self.mode = Mode::Insert,
                    Action::SetToVisual => self.mode = Mode::Visual,
                    Action::SetToVisualLine => self.mode = Mode::VisualLine,
                    Action::SetToVisualBlock => self.mode = Mode::VisualBlock,
                    Action::SetToCommand => self.mode = Mode::Command,
                    Action::Clear => self.mode = Mode::Normal,
                    _ => {}
                }

                if !used_combined {
                    if let Some(op) = self.pending_op.take() {
                        action = action.with_count(motion_count);
                        let combined = resolve_op_motion_action(action, op);
                        self.clear();
                        return combined;
                    }
                    action = action.with_count(motion_count);
                } else {
                    let op_count = self.op_count;
                    self.pending_op = None;
                    action = action.with_count(op_count);
                }

                self.clear();
                return action;
            }
            MatchResult::PendingOp(op_action) => {
                self.op_count = self.take_count();
                self.pending_op = Some(op_action.with_count(self.op_count));
                self.pending_op_sequence = self.key_sequence.clone();
                self.key_sequence.clear();
                return Action::NoOp;
            }
            MatchResult::PrefixMatch => Action::NoOp,
            MatchResult::NoMatch => {
                // Invalid sequence/junk recovery: shift sequence buffer to discard bad prefix
                while !self.key_sequence.is_empty() {
                    self.key_sequence.remove(0);

                    let mut resolved = MatchResult::NoMatch;
                    let mut used_combined = false;
                    if self.pending_op.is_some() && !self.pending_op_sequence.is_empty() {
                        let mut combined: SmallVec<[KeyCombo; 8]> = SmallVec::new();
                        combined.extend(self.pending_op_sequence.iter().cloned());
                        combined.extend(self.key_sequence.iter().cloned());
                        let res = self.try_resolve_sequence(&combined, keymap);
                        if !matches!(res, MatchResult::NoMatch) {
                            resolved = res;
                            used_combined = true;
                        }
                    }
                    if matches!(resolved, MatchResult::NoMatch) {
                        resolved = self.try_resolve(keymap);
                    }

                    if let MatchResult::Complete(mut action) = resolved {
                        let count = self.take_count();
                        match action {
                            Action::SetToInsert => self.mode = Mode::Insert,
                            Action::SetToVisual => self.mode = Mode::Visual,
                            Action::SetToVisualLine => self.mode = Mode::VisualLine,
                            Action::SetToVisualBlock => self.mode = Mode::VisualBlock,
                            Action::SetToCommand => self.mode = Mode::Command,
                            Action::Clear => self.mode = Mode::Normal,
                            _ => {}
                        }
                        if !used_combined {
                            if let Some(op) = self.pending_op.take() {
                                action = action.with_count(count);
                                let combined = resolve_op_motion_action(action, op);
                                self.clear();
                                return combined;
                            }
                            action = action.with_count(count);
                        } else {
                            let op_count = self.op_count;
                            self.pending_op = None;
                            action = action.with_count(op_count);
                        }
                        self.clear();
                        return action;
                    }
                }

                self.key_sequence.clear();
                self.count_buffer.clear();
                Action::NoOp
            }
        }
    }

    fn try_resolve(&self, keymap: &Keymap) -> MatchResult {
        self.try_resolve_sequence(&self.key_sequence, keymap)
    }

    fn try_resolve_sequence(&self, seq: &[KeyCombo], keymap: &Keymap) -> MatchResult {
        // Visual Mode overrides
        if self.mode.is_visual() {
            if let Some(res) = match_sequence_in_map(seq, &keymap.visual_actions) {
                return res;
            }
        }

        // Check Motions
        if let Some(res) = match_sequence_in_map(seq, &keymap.motion_actions) {
            return res;
        }

        // Check Operators
        if self.pending_op.is_none() {
            if let Some(res) = match_sequence_in_map(seq, &keymap.op_actions) {
                if let MatchResult::Complete(op_action) = res {
                    return MatchResult::PendingOp(op_action);
                }
                return res;
            }
        }

        // Check Normal Mode Actions
        if let Some(res) = match_sequence_in_map(seq, &keymap.normal_actions) {
            return res;
        }

        // Check Mode Changes
        if let Some(res) = match_sequence_in_map(seq, &keymap.mode_actions) {
            return res;
        }

        MatchResult::NoMatch
    }

    fn take_count(&mut self) -> u32 {
        if self.count_buffer.is_empty() {
            return 1;
        }
        let count = self.count_buffer.parse::<u32>().unwrap_or(1);
        self.count_buffer.clear();
        count
    }

    fn process_insert_mode(&mut self, combo: KeyCombo, keymap: &Keymap) -> Action {
        self.key_sequence.push(combo.clone());

        if let Some(MatchResult::Complete(action)) =
            match_sequence_in_map(&self.key_sequence, &keymap.insert_actions)
        {
            if action == Action::Clear {
                self.mode = Mode::Normal;
            }
            self.clear();
            return action;
        }

        self.key_sequence.clear();
        if combo.modifiers.is_empty() || combo.modifiers == KeyModifiers::SHIFT {
            if let KeyCode::Char(c) = combo.code {
                return Action::InsertText(c.to_string());
            }
        }
        Action::NoOp
    }
}

enum MatchResult {
    Complete(Action),
    PendingOp(Action),
    PrefixMatch,
    NoMatch,
}

fn match_sequence_in_map(
    buf: &[KeyCombo],
    map: &HashMap<KeyComboSequence, Action>,
) -> Option<MatchResult> {
    let mut has_prefix = false;
    let mut best_match: Option<(&KeyComboSequence, &Action)> = None;

    for (seq, action) in map {
        if buf_matches_pattern(buf, seq) {
            if buf.len() == seq.len() {
                if let Some((best_seq, _)) = best_match {
                    if seq.len() > best_seq.len() {
                        best_match = Some((seq, action));
                    }
                } else {
                    best_match = Some((seq, action));
                }
            } else if buf.len() < seq.len() {
                has_prefix = true;
            }
        }
    }

    if let Some((seq, action)) = best_match {
        let mut final_action = action.clone();
        if seq.items.iter().any(|p| matches!(p, KeyPattern::AnyChar)) {
            if let Some(last) = buf.last() {
                if let KeyCode::Char(ch) = last.code {
                    final_action = final_action.with_char(ch, 1);
                }
            }
        }
        return Some(MatchResult::Complete(final_action));
    }

    if has_prefix {
        return Some(MatchResult::PrefixMatch);
    }

    None
}

fn buf_matches_pattern(buf: &[KeyCombo], pattern: &KeyComboSequence) -> bool {
    if buf.len() > pattern.len() {
        return false;
    }
    buf.iter()
        .zip(pattern.items.iter())
        .all(|(combo, pat)| pat.matches(combo))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combo_parse() {
        // Simple keys
        let esc = KeyCombo::parse("Esc").unwrap();
        assert_eq!(esc.code, KeyCode::Esc);
        assert!(esc.modifiers.is_empty());

        let a = KeyCombo::parse("a").unwrap();
        assert_eq!(a.code, KeyCode::Char('a'));
        assert!(a.modifiers.is_empty());

        let minus = KeyCombo::parse("-").unwrap();
        assert_eq!(minus.code, KeyCode::Char('-'));
        assert!(minus.modifiers.is_empty());

        // Modifiers
        let ctrl_f = KeyCombo::parse("C-f").unwrap();
        assert_eq!(ctrl_f.code, KeyCode::Char('f'));
        assert_eq!(ctrl_f.modifiers, KeyModifiers::CONTROL);

        let alt_x = KeyCombo::parse("M-x").unwrap();
        assert_eq!(alt_x.code, KeyCode::Char('x'));
        assert_eq!(alt_x.modifiers, KeyModifiers::ALT);

        let ctrl_minus = KeyCombo::parse("C--").unwrap();
        assert_eq!(ctrl_minus.code, KeyCode::Char('-'));
        assert_eq!(ctrl_minus.modifiers, KeyModifiers::CONTROL);

        let shift_up = KeyCombo::parse("S-Up").unwrap();
        assert_eq!(shift_up.code, KeyCode::Up);
        assert_eq!(shift_up.modifiers, KeyModifiers::SHIFT);

        // Combined modifiers
        let ctrl_alt_shift_down = KeyCombo::parse("C-M-S-down").unwrap();
        assert_eq!(ctrl_alt_shift_down.code, KeyCode::Down);
        assert_eq!(
            ctrl_alt_shift_down.modifiers,
            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
        );

        // Unknown bindings
        assert!(KeyCombo::parse("UnknownKey").is_err());
        assert!(KeyCombo::parse("X-a").is_err());
    }

    #[test]
    fn test_key_combo_to_string() {
        let combo = KeyCombo::parse("C-M-f").unwrap();
        assert_eq!(combo.to_string(), "C-M-f");

        let shift_a = KeyCombo::parse("S-a").unwrap();
        // Since 'a' is alphabetic, shift modifier is normalized to uppercase char 'A'
        assert_eq!(shift_a.to_string(), "A");
    }

    #[test]
    fn test_key_combo_sequence_parse() {
        // Single characters
        let seq = KeyComboSequence::parse_sequence("gg").unwrap();
        assert_eq!(seq.items.len(), 2);
        assert_eq!(
            seq.items[0],
            KeyPattern::Exact(KeyCombo::parse("g").unwrap())
        );
        assert_eq!(
            seq.items[1],
            KeyPattern::Exact(KeyCombo::parse("g").unwrap())
        );

        let seq_g_minus = KeyComboSequence::parse_sequence("g-").unwrap();
        assert_eq!(seq_g_minus.items.len(), 2);
        assert_eq!(
            seq_g_minus.items[0],
            KeyPattern::Exact(KeyCombo::parse("g").unwrap())
        );
        assert_eq!(
            seq_g_minus.items[1],
            KeyPattern::Exact(KeyCombo::parse("-").unwrap())
        );

        // With wildcards
        let seq_wc = KeyComboSequence::parse_sequence("f{c}").unwrap();
        assert_eq!(seq_wc.items.len(), 2);
        assert_eq!(
            seq_wc.items[0],
            KeyPattern::Exact(KeyCombo::parse("f").unwrap())
        );
        assert_eq!(seq_wc.items[1], KeyPattern::AnyChar);

        // From array/slices
        let seq_arr = ["C-x", "C-s"].into_seq().unwrap();
        assert_eq!(seq_arr.items.len(), 2);
        assert_eq!(
            seq_arr.items[0],
            KeyPattern::Exact(KeyCombo::parse("C-x").unwrap())
        );
        assert_eq!(
            seq_arr.items[1],
            KeyPattern::Exact(KeyCombo::parse("C-s").unwrap())
        );
    }

    #[test]
    fn test_match_sequence_in_map() {
        let mut map = HashMap::new();
        map.bind(
            "gg",
            Action::MoveToStartOfDocument {
                count: 1,
                select: false,
            },
        )
        .unwrap();
        map.bind("g", Action::NoOp).unwrap();

        let keymap = Keymap::new(); // placeholder for map checks

        let g = KeyCombo::parse("g").unwrap();

        // Single 'g' is a prefix to "gg" (since gg has length 2 and starts with g)
        // Note: in our map, "g" is also a complete match (Action::NoOp).
        // Let's test custom match behavior
        let res = match_sequence_in_map(&[g.clone()], &map).unwrap();
        // Since 'gg' is a longer pattern, does it check prefix?
        // Wait, match_sequence_in_map returns Complete for exact match, but also flags prefix if any patterns are longer.
        // Let's verify:
        match res {
            MatchResult::Complete(Action::NoOp) => {}
            _ => panic!("Expected Complete(NoOp)"),
        }

        let gg = vec![g.clone(), g.clone()];
        let res_gg = match_sequence_in_map(&gg, &map).unwrap();
        match res_gg {
            MatchResult::Complete(Action::MoveToStartOfDocument { .. }) => {}
            _ => panic!("Expected Complete(MoveToStartOfDocument)"),
        }
    }

    #[test]
    fn test_input_state_machine_motions() {
        let keymap = Keymap::new();
        let mut sm = InputStateMachine::new();

        // 1. Standalone motion 'j'
        let action = sm.process_key(KeyCombo::parse("j").unwrap(), &keymap);
        assert_eq!(
            action,
            Action::MoveDown {
                count: 1,
                select: false
            }
        );

        // 2. Motion with count '5k'
        sm.process_key(KeyCombo::parse("5").unwrap(), &keymap);
        let action2 = sm.process_key(KeyCombo::parse("k").unwrap(), &keymap);
        assert_eq!(
            action2,
            Action::MoveUp {
                count: 5,
                select: false
            }
        );
    }

    #[test]
    fn test_input_state_machine_operators() {
        let keymap = Keymap::new();
        let mut sm = InputStateMachine::new();

        // 1. Operator followed by motion 'dw'
        let res1 = sm.process_key(KeyCombo::parse("d").unwrap(), &keymap);
        assert_eq!(res1, Action::NoOp);
        assert!(sm.pending_op.is_some());

        let res2 = sm.process_key(KeyCombo::parse("w").unwrap(), &keymap);
        if let Action::DeleteMotion { count, motion } = res2 {
            assert_eq!(count, 1);
            assert_eq!(
                *motion,
                Action::MoveToWord {
                    count: 1,
                    select: false
                }
            );
        } else {
            panic!("Expected DeleteMotion, got {:?}", res2);
        }

        // 2. Line-wise operator 'dd'
        let res3 = sm.process_key(KeyCombo::parse("d").unwrap(), &keymap);
        assert_eq!(res3, Action::NoOp);
        let res4 = sm.process_key(KeyCombo::parse("d").unwrap(), &keymap);
        assert_eq!(res4, Action::DeleteLine { count: 1 });
    }

    #[test]
    fn test_input_state_machine_counts() {
        let keymap = Keymap::new();
        let mut sm = InputStateMachine::new();

        // 1. Operator count and motion count: '2d3w'
        sm.process_key(KeyCombo::parse("2").unwrap(), &keymap);
        sm.process_key(KeyCombo::parse("d").unwrap(), &keymap);
        assert_eq!(sm.op_count, 2);

        sm.process_key(KeyCombo::parse("3").unwrap(), &keymap);
        let action = sm.process_key(KeyCombo::parse("w").unwrap(), &keymap);
        if let Action::DeleteMotion { count, motion } = action {
            assert_eq!(count, 2);
            assert_eq!(
                *motion,
                Action::MoveToWord {
                    count: 3,
                    select: false
                }
            );
        } else {
            panic!("Expected DeleteMotion, got {:?}", action);
        }
    }

    #[test]
    fn test_input_state_machine_wildcard() {
        let keymap = Keymap::new();
        let mut sm = InputStateMachine::new();

        // 'fx' -> MoveToNextCharacter('x')
        sm.process_key(KeyCombo::parse("f").unwrap(), &keymap);
        let action = sm.process_key(KeyCombo::parse("x").unwrap(), &keymap);
        assert_eq!(
            action,
            Action::MoveToNextCharacter {
                count: 1,
                ch: 'x',
                select: false
            }
        );
    }
}
