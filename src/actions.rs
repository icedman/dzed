#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    MoveUp { select: bool },
    MoveDown { select: bool },
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
    DeleteCurrentLine,

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
