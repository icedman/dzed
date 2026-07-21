use onig::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectInKind {
    Word,
    Pargraph,
    Curly,
    Parenthesis,
    Square,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    SetNormalMode,
    SetInsertMode,
    SetVisualMode,
    SetVisualLineMode,
    SetVisualBlockMode,
    SetCommandMode {
        search: bool,
        pattern: bool,
    },

    MoveUp {
        select: bool,
        count: u32,
    },
    MoveDown {
        select: bool,
        count: u32,
    },
    MoveLeft {
        select: bool,
        count: u32,
    },
    MoveRight {
        select: bool,
        count: u32,
    },
    MoveToPreviousWord {
        select: bool,
        count: u32,
    },
    MoveToNextWord {
        select: bool,
        count: u32,
    },
    MoveToPreviousWordEnd {
        select: bool,
        count: u32,
    },
    MoveToNextWordEnd {
        select: bool,
        count: u32,
    },
    MoveToStartOfDocument {
        select: bool,
    },
    MoveToEndOfDocument {
        select: bool,
    },
    MoveToStartOfLine {
        select: bool,
    },
    MoveToStartOfLineNonSpace {
        select: bool,
    },
    MoveToEndOfLine {
        select: bool,
    },
    MoveToLine {
        select: bool,
        line: u32,
    },
    MoveToPreviousParagraph {
        select: bool,
        count: u32,
    },
    MoveToNextParagraph {
        select: bool,
        count: u32,
    },
    MoveToPreviousCharacter {
        select: bool,
        count: u32,
        char: char,
    },
    MoveToNextCharacter {
        select: bool,
        count: u32,
        char: char,
    },
    MoveToPreviousMatch {
        search: String,
        pattern: bool,
    },
    MoveToNextMatch {
        search: String,
        pattern: bool,
    },

    InsertText(String),
    InsertNewLine,
    InsertTab,

    DeleteText {
        count: usize,
    },
    Backspace,
    Delete {
        count: u32,
    },
    DeleteCurrentLine {
        count: u32,
    },
    DeleteMotion {
        count: u32,
        motion: Box<Action>,
    },
    Change,
    ChangeMotion {
        count: u32,
        motion: Box<Action>,
    },

    Indent,
    Unindent,

    Undo {
        count: u32,
    },
    Redo {
        count: u32,
    },

    SelectIn {
        kind: SelectInKind,
    },

    ClearCursors,

    NoOp, // unmapped key
}

/*
Vim Actions to Implement:

Movement (Basic):
- [x] h, j, k, l (Left, Down, Up, Right)
- [x] w, b (Word forward, Word backward)
- [x] e, ge (End of word, End of word backward)
- [ ] W, B, E (Space-separated word movements)
- [x] 0, $ (Start of line, End of line)
- [x] ^ (First non-blank character)
- [x] gg, G (Start of document, End of document)
- [x] { , } (Previous/Next paragraph)
- [ ] % (Jump to matching bracket)

Movement (Advanced):
- [x] f{char}, F{char} (Find character forward/backward)
- [ ] t{char}, T{char} (Till character forward/backward)
- [ ] ; , (Repeat last f/F/t/T search)
- [ ] H, M, L (High, Middle, Low of screen)
- [ ] Ctrl-u, Ctrl-d (Half page up/down)
- [ ] Ctrl-b, Ctrl-f (Full page up/down)

Editing:
- [x] i, I (InsertText handles this after mode switch)
- [ ] a, A (Append after cursor, Append at end of line)
- [x] o, O (Open new line below/above - via InsertNewLine)
- [ ] r{char}, R (Replace single character, Enter Replace mode)
- [ ] s, S (Substitute character, Substitute line)
- [x] x, X (Delete character - via Delete)
- [x] d{motion}, dd, D (DLelete with motion, Delete line, Delete to end of line - via DeleteCurrentLine)
- [ ] c{motion}, cc, C (Change with motion, Change line, Change to end of line)
- [ ] y{motion}, yy, Y (Yank/Copy with motion, Yank line)
- [ ] p, P (Put/Paste after/before cursor)
- [x] u, Ctrl-r (Undo, Redo)
- [ ] . (Repeat last change)
- [x] >, < (Indent, Unindent)

Search:
- [ ] /pattern, ?pattern (Search forward/backward)
- [ ] n, N (Next/Previous search result)
- [ ] *, # (Search for word under cursor forward/backward)

Visual Mode:
- [ ] v, V, Ctrl-v (Character, Line, and Block visual modes)
- [ ] gv (Reselect last visual selection)

Command Mode:
- [ ] :w (Save), :q (Quit), :wq (Save and Quit)
- [ ] :%s/old/new/g (Global search and replace)
*/
