#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    MoveUp { select: bool },
    MoveDown { select: bool },
    MoveUpCount { select: bool, count: u32 },
    MoveDownCount { select: bool, count: u32 },
    MoveLeft { select: bool },
    MoveRight { select: bool },
    MoveToPreviousWord { select: bool },
    MoveToNextWord { select: bool },
    MoveToStartOfDocument { select: bool },
    MoveToEndOfDocument { select: bool },
    MoveToStartOfLine { select: bool },
    MoveToEndOfLine { select: bool },
    MoveToPreviousParagraph { select: bool },
    MoveToNextParagraph { select: bool },

    InsertText(String),
    InsertNewLine,
    InsertTab,

    DeleteText { count: usize },
    Backspace,
    Delete,
    DeleteCurrentWord,
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
