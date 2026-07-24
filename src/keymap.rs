use crate::actions::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyCombo {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
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
            match self.code {
                KeyCode::Char(_) => {} // Shift is usually reflected in the char itself
                _ => s.push_str("S-"),
            }
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
            KeyCode::Delete => s.push_str("Delete"),
            KeyCode::Insert => s.push_str("Insert"),
            KeyCode::BackTab => s.push_str("BackTab"),
            KeyCode::F(n) => s.push_str(&format!("F{}", n)),
            _ => s.push_str(&format!("{:?}", self.code)),
        }
        s
    }
}

impl From<&KeyEvent> for KeyCombo {
    fn from(event: &KeyEvent) -> Self {
        let mut code = event.code;
        let mut modifiers = event.modifiers;

        if modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = code {
                code = KeyCode::Char(c.to_ascii_uppercase());
                // If it's a character and we have Shift, we've normalized the char
                // so we can often consider Shift "consumed".
                if c.is_ascii_alphabetic() {
                    modifiers.remove(KeyModifiers::SHIFT);
                }
            }
        }

        Self { code, modifiers }
    }
}

pub struct Keymap {
    pub op_actions: HashMap<String, Action>,
    pub motion_actions: HashMap<String, Action>,
    pub mode_actions: HashMap<String, Action>,
    pub normal_actions: HashMap<String, Action>,
    pub insert_actions: HashMap<String, Action>,
    pub visual_actions: HashMap<String, Action>,
}

impl Keymap {
    pub fn new() -> Self {
        let mut op_actions = HashMap::new();
        let mut motion_actions = HashMap::new();
        let mut normal_actions = HashMap::new();
        let mut mode_actions = HashMap::new();
        let mut insert_actions = HashMap::new();
        let mut visual_actions = HashMap::new();

        // operators
        op_actions.insert("d".to_string(), Action::Delete { count: 1 });
        op_actions.insert("c".to_string(), Action::Change { count: 1 });
        op_actions.insert("y".to_string(), Action::Yank { count: 1 });

        // motions
        motion_actions.insert(
            "w".to_string(),
            Action::MoveToWord {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "e".to_string(),
            Action::MoveToWordEnd {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "b".to_string(),
            Action::MoveToPreviousWord {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "ge".to_string(),
            Action::MoveToPreviousWordEnd {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "W".to_string(),
            Action::MoveToBigWord {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "B".to_string(),
            Action::MoveToPreviousBigWord {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "E".to_string(),
            Action::MoveToBigWordEnd {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "h".to_string(),
            Action::MoveLeft {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "l".to_string(),
            Action::MoveRight {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "k".to_string(),
            Action::MoveUp {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "j".to_string(),
            Action::MoveDown {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "Left".to_string(),
            Action::MoveLeft {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "Right".to_string(),
            Action::MoveRight {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "Up".to_string(),
            Action::MoveUp {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "Down".to_string(),
            Action::MoveDown {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "gg".to_string(),
            Action::MoveToStartOfDocument {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "G".to_string(),
            Action::MoveToEndOfDocument {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "0".to_string(),
            Action::MoveToStartOfLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "^".to_string(),
            Action::MoveToStartOfLineNonSpace {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "$".to_string(),
            Action::MoveToEndOfLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "-".to_string(),
            Action::MoveToStartOfPreviousLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "+".to_string(),
            Action::MoveToStartOfNextLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "g-".to_string(),
            Action::MoveToEndOfPreviousLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "g+".to_string(),
            Action::MoveToEndOfNextLine {
                count: 1,
                select: false,
            },
        );

        motion_actions.insert(
            "H".to_string(),
            Action::MoveToScreenTop {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "M".to_string(),
            Action::MoveToScreenMiddle {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "L".to_string(),
            Action::MoveToScreenBottom {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "{".to_string(),
            Action::MoveToPreviousParagraph {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "}".to_string(),
            Action::MoveToNextParagraph {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "(".to_string(),
            Action::MoveToPreviousSentence {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            ")".to_string(),
            Action::MoveToNextSentence {
                count: 1,
                select: false,
            },
        );

        motion_actions.insert(
            "f{c}".to_string(),
            Action::MoveToNextCharacter {
                count: 1,
                select: false,
                ch: '?',
            },
        );
        motion_actions.insert(
            "F{c}".to_string(),
            Action::MoveToPreviousCharacter {
                count: 1,
                select: false,
                ch: '?',
            },
        );

        motion_actions.insert("C-f".to_string(), Action::ScrollForward { count: 1 });
        motion_actions.insert("C-b".to_string(), Action::ScrollBackward { count: 1 });
        motion_actions.insert("C-d".to_string(), Action::ScrollHalfPageDown { count: 1 });
        motion_actions.insert("C-u".to_string(), Action::ScrollHalfPageUp { count: 1 });
        motion_actions.insert("C-e".to_string(), Action::ScrollLineDown { count: 1 });
        motion_actions.insert("C-y".to_string(), Action::ScrollLineUp { count: 1 });

        motion_actions.insert("|".to_string(), Action::MoveToColumn { count: 1 });

        motion_actions.insert("/".to_string(), Action::SearchForward { count: 1 });
        motion_actions.insert("?".to_string(), Action::SearchBackward { count: 1 });
        motion_actions.insert("n".to_string(), Action::SearchNext { count: 1 });
        motion_actions.insert("N".to_string(), Action::SearchPrevious { count: 1 });

        motion_actions.insert(
            "End".to_string(),
            Action::MoveToEndOfLine {
                count: 1,
                select: false,
            },
        );
        motion_actions.insert(
            "Home".to_string(),
            Action::MoveToStartOfLine {
                count: 1,
                select: false,
            },
        );

        // normal
        normal_actions.insert("dd".to_string(), Action::DeleteLine { count: 1 });
        normal_actions.insert("cc".to_string(), Action::ChangeLine { count: 1 });
        normal_actions.insert("yy".to_string(), Action::YankLine { count: 1 });

        normal_actions.insert("x".to_string(), Action::DeleteChar { count: 1 });
        normal_actions.insert("X".to_string(), Action::DeleteCharBefore { count: 1 });
        normal_actions.insert("p".to_string(), Action::Put { count: 1 });
        normal_actions.insert("P".to_string(), Action::PutBefore { count: 1 });
        normal_actions.insert("J".to_string(), Action::JoinLines { count: 1 });
        normal_actions.insert("u".to_string(), Action::Undo { count: 1 });
        normal_actions.insert("C-r".to_string(), Action::Redo { count: 1 });
        normal_actions.insert(".".to_string(), Action::Repeat { count: 1 });
        normal_actions.insert(">".to_string(), Action::Indent { count: 1 });
        normal_actions.insert("<".to_string(), Action::Outdent { count: 1 });
        normal_actions.insert("~".to_string(), Action::ChangeCase { count: 1 });

        normal_actions.insert("Delete".to_string(), Action::DeleteChar { count: 1 });
        normal_actions.insert(
            "Backspace".to_string(),
            Action::MoveLeft {
                count: 1,
                select: false,
            },
        );

        normal_actions.insert("Esc".to_string(), Action::Clear);

        // change mode -- only at normal
        mode_actions.insert("i".to_string(), Action::SetToInsert);
        mode_actions.insert(":".to_string(), Action::SetToCommand);
        mode_actions.insert("v".to_string(), Action::SetToVisual);
        mode_actions.insert("V".to_string(), Action::SetToVisualLine);
        mode_actions.insert("C-v".to_string(), Action::SetToVisualBlock);
        mode_actions.insert(":".to_string(), Action::SetToCommand);

        // insert mode
        insert_actions.insert(
            "Left".to_string(),
            Action::MoveLeft {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert(
            "Right".to_string(),
            Action::MoveRight {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert(
            "Up".to_string(),
            Action::MoveUp {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert(
            "Down".to_string(),
            Action::MoveDown {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert(
            "PageUp".to_string(),
            Action::MovePageUp {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert(
            "Down".to_string(),
            Action::MoveDown {
                count: 1,
                select: false,
            },
        );
        insert_actions.insert("Esc".to_string(), Action::Clear);
        insert_actions.insert("Enter".to_string(), Action::InsertNewLine { count: 1 });
        insert_actions.insert("Tab".to_string(), Action::InsertTab);
        insert_actions.insert(
            "Backspace".to_string(),
            Action::DeleteCharBefore { count: 1 },
        );
        insert_actions.insert("Delete".to_string(), Action::DeleteChar { count: 1 });
        insert_actions.insert("{c}".to_string(), Action::InsertText("".to_string()));

        visual_actions.insert("Esc".to_string(), Action::Clear);

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
