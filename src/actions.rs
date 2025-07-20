// actions.rs
#[derive(Debug, Clone)]
pub enum Action {
    MoveUp { select: bool, count: usize },
    MoveDown { select: bool, count: usize },
    MoveLeft { select: bool },
    MoveRight { select: bool },
    MoveToPreviousWord { select: bool },
    MoveToNextWord { select: bool },
    MoveToStartOfDocument { select: bool },
    MoveToEndOfDocument { select: bool },
    MoveToStartOfLine { select: bool },
    MoveToEndOfLine { select: bool },

    InsertText(String),
    DeleteText { count: usize },
    InsertNewLine,
    InsertTab,

    Indent,
    Unindent,

    Undo,
    Redo,

    SelectCurrentWord,
    SelectNextSameWord(String),

    ClearCursors,

    NoOp, // unmapped key
}
