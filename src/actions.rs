use crate::actions::Mode::{Visual, VisualBlock, VisualLine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Command,
}

impl Mode {
    pub fn is_visual(&self) -> bool {
        matches!(self, Mode::Visual | Mode::VisualLine | Mode::VisualBlock)
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
            Mode::Visual => "Visual",
            Mode::VisualLine => "V-Line",
            Mode::VisualBlock => "V-Block",
            Mode::Command => "Command",
        };
        write!(f, "{}", name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    // OPTS
    NoOp,
    Clear,
    Delete { count: u32 },
    Change { count: u32 },
    Yank { count: u32 },

    // MOTIONS
    StandBy { count: u32, select: bool },

    MoveLeft { count: u32, select: bool },
    MoveRight { count: u32, select: bool },
    MoveUp { count: u32, select: bool },
    MoveDown { count: u32, select: bool },

    MovePageUp { count: u32, select: bool },
    MovePageDown { count: u32, select: bool },

    MoveToWord { count: u32, select: bool },
    MoveToPreviousWord { count: u32, select: bool },
    MoveToWordEnd { count: u32, select: bool },
    MoveToPreviousWordEnd { count: u32, select: bool },

    MoveToBigWord { count: u32, select: bool },
    MoveToPreviousBigWord { count: u32, select: bool },
    MoveToBigWordEnd { count: u32, select: bool },

    MoveToStartOfDocument { count: u32, select: bool },
    MoveToEndOfDocument { count: u32, select: bool },
    MoveToStartOfLine { count: u32, select: bool },
    MoveToStartOfLineNonSpace { count: u32, select: bool },
    MoveToEndOfLine { count: u32, select: bool },
    MoveToStartOfPreviousLine { count: u32, select: bool },
    MoveToEndOfPreviousLine { count: u32, select: bool },
    MoveToStartOfNextLine { count: u32, select: bool },
    MoveToEndOfNextLine { count: u32, select: bool },

    MoveToScreenTop { count: u32, select: bool },
    MoveToScreenMiddle { count: u32, select: bool },
    MoveToScreenBottom { count: u32, select: bool },
    MoveToPreviousParagraph { count: u32, select: bool },
    MoveToNextParagraph { count: u32, select: bool },
    MoveToPreviousSentence { count: u32, select: bool },
    MoveToNextSentence { count: u32, select: bool },

    MoveToNextCharacter { count: u32, ch: char, select: bool },
    MoveToPreviousCharacter { count: u32, ch: char, select: bool },

    MoveWithinCharacter { count: u32, ch: char },
    MoveAroundCharacter { count: u32, ch: char },

    ScrollForward { count: u32 },
    ScrollBackward { count: u32 },
    ScrollHalfPageDown { count: u32 },
    ScrollHalfPageUp { count: u32 },
    ScrollLineDown { count: u32 },
    ScrollLineUp { count: u32 },

    MoveToColumn { count: u32 },

    SearchForward { count: u32 },
    SearchBackward { count: u32 },
    SearchNext { count: u32 },
    SearchPrevious { count: u32 },

    // OPT+MOTION
    DeleteMotion { count: u32, motion: Box<Action> },
    ChangeMotion { count: u32, motion: Box<Action> },
    YankMotion { count: u32, motion: Box<Action> },

    // NORMAL
    DeleteLine { count: u32 },
    ChangeLine { count: u32 },
    YankLine { count: u32 },
    JoinLines { count: u32 },
    DeleteChar { count: u32 },
    DeleteCharBefore { count: u32 },
    Put { count: u32 },
    PutBefore { count: u32 },
    Undo { count: u32 },
    Redo { count: u32 },
    Repeat { count: u32 },
    Indent { count: u32 },
    Outdent { count: u32 },
    ChangeCase { count: u32 },

    // MODE SELECT
    SetToNormal,
    SetToInsert,
    SetToVisual,
    SetToVisualLine,
    SetToVisualBlock,
    SetToCommand,

    // INSERT
    InsertNewLine { count: u32 },
    InsertText(String),
    InsertNewLineMotion { count: u32, motion: Box<Action> },
    InsertTab,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::NoOp => write!(f, "None"),
            Action::Clear => write!(f, "Clear"),
            Action::Delete { count } => write!(f, "Delete({})", count),
            Action::Change { count } => write!(f, "Change({})", count),
            Action::Yank { count } => write!(f, "Yank({})", count),
            Action::MoveToWord { count, .. } => write!(f, "MoveToWord({})", count),
            Action::MoveToPreviousWord { count, .. } => write!(f, "MoveToPreviousWord({})", count),
            Action::MoveToWordEnd { count, .. } => write!(f, "MoveToWordEnd({})", count),
            Action::MoveToPreviousWordEnd { count, .. } => {
                write!(f, "MoveToPreviousWordEnd({})", count)
            }
            Action::MoveToBigWord { count, .. } => write!(f, "MoveToBigWord({})", count),
            Action::MoveToPreviousBigWord { count, .. } => {
                write!(f, "MoveToPrevBigWord({})", count)
            }
            Action::MoveToBigWordEnd { count, .. } => write!(f, "MoveToBigWordEnd({})", count),
            Action::MoveToStartOfDocument { count, .. } => write!(f, "MoveToStartOfDoc({})", count),
            Action::MoveToEndOfDocument { count, .. } => write!(f, "MoveToEndOfDoc({})", count),
            Action::MoveToStartOfLine { count, .. } => write!(f, "MoveToStartOfLine({})", count),
            Action::MoveToStartOfLineNonSpace { count, .. } => {
                write!(f, "MoveToStartOfLineNonSpace({})", count)
            }
            Action::MoveToEndOfLine { count, .. } => write!(f, "MoveToEndOfLine({})", count),
            Action::MoveToStartOfPreviousLine { count, .. } => {
                write!(f, "MoveToStartOfPrevLine({})", count)
            }
            Action::MoveToEndOfPreviousLine { count, .. } => {
                write!(f, "MoveToEndOfPrevLine({})", count)
            }
            Action::MoveToStartOfNextLine { count, .. } => {
                write!(f, "MoveToStartOfNextLine({})", count)
            }
            Action::MoveToEndOfNextLine { count, .. } => {
                write!(f, "MoveToEndOfNextLine({})", count)
            }
            Action::MoveToScreenTop { count, .. } => write!(f, "MoveToScreenTop({})", count),
            Action::MoveToScreenMiddle { count, .. } => write!(f, "MoveToScreenMiddle({})", count),
            Action::MoveToScreenBottom { count, .. } => write!(f, "MoveToScreenBottom({})", count),
            Action::MoveToPreviousParagraph { count, .. } => write!(f, "MoveToPrevPara({})", count),
            Action::MoveToNextParagraph { count, .. } => write!(f, "MoveToNextPara({})", count),
            Action::MoveToPreviousSentence { count, .. } => write!(f, "MoveToPrevSent({})", count),
            Action::MoveToNextSentence { count, .. } => write!(f, "MoveToNextSent({})", count),
            Action::ScrollForward { count } => write!(f, "ScrollForward({})", count),
            Action::ScrollBackward { count } => write!(f, "ScrollBackward({})", count),
            Action::ScrollHalfPageDown { count } => write!(f, "ScrollHalfPageDown({})", count),
            Action::ScrollHalfPageUp { count } => write!(f, "ScrollHalfPageUp({})", count),
            Action::ScrollLineDown { count } => write!(f, "ScrollLineDown({})", count),
            Action::ScrollLineUp { count } => write!(f, "ScrollLineUp({})", count),
            Action::MoveToColumn { count } => write!(f, "MoveToColumn({})", count),
            Action::SearchForward { count } => write!(f, "SearchForward {}", count),
            Action::SearchBackward { count } => write!(f, "SearchBackward {}", count),
            Action::SearchNext { count } => write!(f, "SearchNext({})", count),
            Action::SearchPrevious { count } => write!(f, "SearchPrev({})", count),
            Action::StandBy { count, .. } => write!(f, "StandBy({})", count),
            Action::MoveLeft { count, .. } => write!(f, "MoveLeft({})", count),
            Action::MoveRight { count, .. } => write!(f, "MoveRight({})", count),
            Action::MoveUp { count, .. } => write!(f, "MoveUp({})", count),
            Action::MoveDown { count, .. } => write!(f, "MoveDown({})", count),
            Action::MovePageUp { count, .. } => write!(f, "MovePageUp({})", count),
            Action::MovePageDown { count, .. } => write!(f, "MovePageDown({})", count),
            Action::MoveToNextCharacter { count, ch, .. } => {
                write!(f, "MoveToNextCharacter({} {})", count, ch)
            }
            Action::MoveToPreviousCharacter { count, ch, .. } => {
                write!(f, "MoveToPreviousCharacter({} {})", count, ch)
            }
            Action::MoveWithinCharacter { count, ch, .. } => {
                write!(f, "MoveWithinCharacter({} {})", count, ch)
            }
            Action::MoveAroundCharacter { count, ch, .. } => {
                write!(f, "MoveAroundCharacter({} {})", count, ch)
            }
            Action::DeleteMotion { count, motion } => {
                write!(f, "DeleteMotion({}, {})", count, motion)
            }
            Action::ChangeMotion { count, motion } => {
                write!(f, "ChangeMotion({}, {})", count, motion)
            }
            Action::YankMotion { count, motion } => {
                write!(f, "YankMotion({}, {})", count, motion)
            }
            Action::DeleteLine { count } => write!(f, "DeleteLine({})", count),
            Action::ChangeLine { count } => write!(f, "ChangeLine({})", count),
            Action::YankLine { count } => write!(f, "YankLine({})", count),
            Action::JoinLines { count } => write!(f, "JoinLines({})", count),
            Action::DeleteChar { count } => write!(f, "DeleteChar({})", count),
            Action::DeleteCharBefore { count } => write!(f, "DeleteCharBefore({})", count),
            Action::Put { count } => write!(f, "Put({})", count),
            Action::PutBefore { count } => write!(f, "PutBefore({})", count),
            Action::Undo { count } => write!(f, "Undo({})", count),
            Action::Redo { count } => write!(f, "Redo({})", count),
            Action::Repeat { count } => write!(f, "Repeat({})", count),
            Action::Indent { count } => write!(f, "Indent({})", count),
            Action::Outdent { count } => write!(f, "Outdent({})", count),
            Action::ChangeCase { count } => write!(f, "ChangeCase({})", count),
            Action::SetToNormal => write!(f, "SetNormal"),
            Action::SetToInsert => write!(f, "SetInsert"),
            Action::SetToVisual => write!(f, "SetVisual"),
            Action::SetToVisualLine => write!(f, "SetV-Line"),
            Action::SetToVisualBlock => write!(f, "SetV-Block"),
            Action::SetToCommand => write!(f, "SetCommand"),
            Action::InsertNewLine { count } => write!(f, "InsertNewLine({})", count),
            Action::InsertText(s) => write!(f, "InsertText({})", s),
            Action::InsertNewLineMotion { count, motion } => {
                write!(f, "InsertNewLineMotion({}, {})", count, motion)
            }
            Action::InsertTab => write!(f, "InsertTab"),
        }
    }
}

impl Action {
    pub fn with_select(self, select: bool) -> Self {
        match self {
            Action::StandBy { count, .. } => Action::StandBy { count, select },
            Action::MoveLeft { count, .. } => Action::MoveLeft { count, select },
            Action::MoveRight { count, .. } => Action::MoveRight { count, select },
            Action::MoveUp { count, .. } => Action::MoveUp { count, select },
            Action::MoveDown { count, .. } => Action::MoveDown { count, select },
            _ => Action::NoOp,
        }
    }

    pub fn with_count(self, count: u32) -> Self {
        match self {
            Action::Delete { .. } => Action::Delete { count },
            Action::Change { .. } => Action::Change { count },
            Action::Yank { .. } => Action::Yank { count },
            Action::MoveToWord { .. } => Action::MoveToWord {
                count,
                select: false,
            },
            Action::MoveToPreviousWord { .. } => Action::MoveToPreviousWord {
                count,
                select: false,
            },
            Action::MoveToWordEnd { .. } => Action::MoveToWordEnd {
                count,
                select: false,
            },
            Action::MoveToPreviousWordEnd { .. } => Action::MoveToPreviousWordEnd {
                count,
                select: false,
            },
            Action::MoveToBigWord { .. } => Action::MoveToBigWord {
                count,
                select: false,
            },
            Action::MoveToPreviousBigWord { .. } => Action::MoveToPreviousBigWord {
                count,
                select: false,
            },
            Action::MoveToBigWordEnd { .. } => Action::MoveToBigWordEnd {
                count,
                select: false,
            },
            Action::MoveToStartOfDocument { .. } => Action::MoveToStartOfDocument {
                count,
                select: false,
            },
            Action::MoveToEndOfDocument { .. } => Action::MoveToEndOfDocument {
                count,
                select: false,
            },
            Action::MoveToStartOfLine { .. } => Action::MoveToStartOfLine {
                count,
                select: false,
            },
            Action::MoveToStartOfLineNonSpace { .. } => Action::MoveToStartOfLineNonSpace {
                count,
                select: false,
            },
            Action::MoveToEndOfLine { .. } => Action::MoveToEndOfLine {
                count,
                select: false,
            },
            Action::MoveToStartOfPreviousLine { .. } => Action::MoveToStartOfPreviousLine {
                count,
                select: false,
            },
            Action::MoveToEndOfPreviousLine { .. } => Action::MoveToEndOfPreviousLine {
                count,
                select: false,
            },
            Action::MoveToStartOfNextLine { .. } => Action::MoveToStartOfNextLine {
                count,
                select: false,
            },
            Action::MoveToEndOfNextLine { .. } => Action::MoveToEndOfNextLine {
                count,
                select: false,
            },
            Action::MoveToScreenTop { .. } => Action::MoveToScreenTop {
                count,
                select: false,
            },
            Action::MoveToScreenMiddle { .. } => Action::MoveToScreenMiddle {
                count,
                select: false,
            },
            Action::MoveToScreenBottom { .. } => Action::MoveToScreenBottom {
                count,
                select: false,
            },
            Action::MoveToPreviousParagraph { .. } => Action::MoveToPreviousParagraph {
                count,
                select: false,
            },
            Action::MoveToNextParagraph { .. } => Action::MoveToNextParagraph {
                count,
                select: false,
            },
            Action::MoveToPreviousSentence { .. } => Action::MoveToPreviousSentence {
                count,
                select: false,
            },
            Action::MoveToNextSentence { .. } => Action::MoveToNextSentence {
                count,
                select: false,
            },
            Action::MoveToNextCharacter { .. } => Action::MoveToNextCharacter {
                count,
                ch: '?',
                select: false,
            },
            Action::MoveToPreviousCharacter { .. } => Action::MoveToPreviousCharacter {
                count,
                ch: '?',
                select: false,
            },
            Action::MoveWithinCharacter { .. } => Action::MoveWithinCharacter { count, ch: '?' },
            Action::MoveAroundCharacter { .. } => Action::MoveAroundCharacter { count, ch: '?' },

            Action::ScrollForward { .. } => Action::ScrollForward { count },
            Action::ScrollBackward { .. } => Action::ScrollBackward { count },
            Action::ScrollHalfPageDown { .. } => Action::ScrollHalfPageDown { count },
            Action::ScrollHalfPageUp { .. } => Action::ScrollHalfPageUp { count },
            Action::ScrollLineDown { .. } => Action::ScrollLineDown { count },
            Action::ScrollLineUp { .. } => Action::ScrollLineUp { count },
            Action::MoveToColumn { .. } => Action::MoveToColumn { count },
            Action::SearchForward { .. } => Action::SearchForward { count },
            Action::SearchBackward { .. } => Action::SearchBackward { count },
            Action::SearchNext { .. } => Action::SearchNext { count },
            Action::SearchPrevious { .. } => Action::SearchPrevious { count },
            Action::StandBy { .. } => Action::StandBy {
                count,
                select: false,
            },
            Action::MoveLeft { .. } => Action::MoveLeft {
                count,
                select: false,
            },
            Action::MoveRight { .. } => Action::MoveRight {
                count,
                select: false,
            },
            Action::MoveUp { .. } => Action::MoveUp {
                count,
                select: false,
            },
            Action::MoveDown { .. } => Action::MoveDown {
                count,
                select: false,
            },
            Action::MovePageUp { .. } => Action::MovePageUp {
                count,
                select: false,
            },
            Action::MovePageDown { .. } => Action::MovePageDown {
                count,
                select: false,
            },
            Action::DeleteLine { .. } => Action::DeleteLine { count },
            Action::ChangeLine { .. } => Action::ChangeLine { count },
            Action::YankLine { .. } => Action::YankLine { count },
            Action::JoinLines { .. } => Action::JoinLines { count },
            Action::DeleteChar { .. } => Action::DeleteChar { count },
            Action::DeleteCharBefore { .. } => Action::DeleteCharBefore { count },
            Action::Put { .. } => Action::Put { count },
            Action::PutBefore { .. } => Action::PutBefore { count },
            Action::Undo { .. } => Action::Undo { count },
            Action::Redo { .. } => Action::Redo { count },
            Action::Repeat { .. } => Action::Repeat { count },
            Action::Indent { .. } => Action::Indent { count },
            Action::Outdent { .. } => Action::Outdent { count },
            Action::ChangeCase { .. } => Action::ChangeCase { count },
            Action::DeleteMotion { motion, .. } => Action::DeleteMotion { count, motion },
            Action::ChangeMotion { motion, .. } => Action::ChangeMotion { count, motion },
            Action::YankMotion { motion, .. } => Action::YankMotion { count, motion },
            Action::SetToNormal => Action::SetToNormal,
            Action::SetToInsert => Action::SetToInsert,
            Action::SetToVisual => Action::SetToVisual,
            Action::SetToVisualLine => Action::SetToVisualLine,
            Action::SetToVisualBlock => Action::SetToVisualBlock,
            Action::SetToCommand => Action::SetToCommand,
            Action::InsertNewLine { .. } => Action::InsertNewLine { count },
            Action::InsertText(s) => Action::InsertText(s),
            Action::InsertNewLineMotion { motion, .. } => Action::InsertNewLineMotion {
                count,
                motion: motion,
            },
            Action::InsertTab => Action::InsertTab,
            Action::Clear => Action::Clear,
            Action::NoOp => Action::NoOp,
        }
    }

    pub fn with_char(self, ch: char, count: u32) -> Self {
        match self {
            Action::MoveToNextCharacter { .. } => Action::MoveToNextCharacter {
                select: false,
                ch,
                count,
            },
            Action::MoveToPreviousCharacter { .. } => Action::MoveToPreviousCharacter {
                select: false,
                ch,
                count,
            },
            Action::InsertText(_) => Action::InsertText(ch.to_string()),
            _ => Action::NoOp,
        }
    }

    pub fn count(&self) -> u32 {
        match self {
            Action::Delete { count } => *count,
            Action::Change { count } => *count,
            Action::Yank { count } => *count,
            Action::MoveToWord { count, .. } => *count,
            Action::MoveToPreviousWord { count, .. } => *count,
            Action::MoveToWordEnd { count, .. } => *count,
            Action::MoveToPreviousWordEnd { count, .. } => *count,
            Action::MoveToBigWord { count, .. } => *count,
            Action::MoveToPreviousBigWord { count, .. } => *count,
            Action::MoveToBigWordEnd { count, .. } => *count,
            Action::MoveToStartOfDocument { count, .. } => *count,
            Action::MoveToEndOfDocument { count, .. } => *count,
            Action::MoveToStartOfLine { count, .. } => *count,
            Action::MoveToStartOfLineNonSpace { count, .. } => *count,
            Action::MoveToEndOfLine { count, .. } => *count,
            Action::MoveToStartOfPreviousLine { count, .. } => *count,
            Action::MoveToEndOfPreviousLine { count, .. } => *count,
            Action::MoveToStartOfNextLine { count, .. } => *count,
            Action::MoveToEndOfNextLine { count, .. } => *count,
            Action::MoveToScreenTop { count, .. } => *count,
            Action::MoveToScreenMiddle { count, .. } => *count,
            Action::MoveToScreenBottom { count, .. } => *count,
            Action::MoveToPreviousParagraph { count, .. } => *count,
            Action::MoveToNextParagraph { count, .. } => *count,
            Action::MoveToPreviousSentence { count, .. } => *count,
            Action::MoveToNextSentence { count, .. } => *count,
            Action::ScrollForward { count } => *count,
            Action::ScrollBackward { count } => *count,
            Action::ScrollHalfPageDown { count } => *count,
            Action::ScrollHalfPageUp { count } => *count,
            Action::ScrollLineDown { count } => *count,
            Action::ScrollLineUp { count } => *count,
            Action::MoveToColumn { count } => *count,
            Action::SearchForward { count } => *count,
            Action::SearchBackward { count } => *count,
            Action::SearchNext { count } => *count,
            Action::SearchPrevious { count } => *count,
            Action::MoveLeft { count, .. } => *count,
            Action::MoveRight { count, .. } => *count,
            Action::MoveUp { count, .. } => *count,
            Action::MoveDown { count, .. } => *count,
            Action::MovePageUp { count, .. } => *count,
            Action::MovePageDown { count, .. } => *count,
            Action::DeleteLine { count } => *count,
            Action::ChangeLine { count } => *count,
            Action::YankLine { count } => *count,
            Action::JoinLines { count } => *count,
            Action::DeleteChar { count } => *count,
            Action::DeleteCharBefore { count } => *count,
            Action::Put { count } => *count,
            Action::PutBefore { count } => *count,
            Action::Undo { count } => *count,
            Action::Redo { count } => *count,
            Action::Repeat { count } => *count,
            Action::Indent { count } => *count,
            Action::Outdent { count } => *count,
            Action::ChangeCase { count } => *count,
            Action::DeleteMotion { count, .. } => *count,
            Action::ChangeMotion { count, .. } => *count,
            Action::YankMotion { count, .. } => *count,
            Action::InsertNewLine { count } => *count,
            Action::InsertNewLineMotion { count, .. } => *count,
            _ => 1,
        }
    }
}
