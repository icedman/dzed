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
    InsertNewLine,
    InsertTab,

    DeleteText { count: usize },
    Backspace,
    Delete,

    Indent,
    Unindent,

    Undo,
    Redo,

    SelectWord,
    SelectNext(String),
    SelectPrevious(String),

    ClearCursors,

    NoOp, // unmapped key
}
