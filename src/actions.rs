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
    Paragraph,
    Curly,
    Parenthesis,
    Square,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    SetNormalMode,
    SetInsertMode,
    SetInsertModeMotion {
        motion: Box<Action>,
    },
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
    MoveToStartOfPreviousLine {
        select: bool,
    },
    MoveToEndOfPreviousLine {
        select: bool,
    },
    MoveToStartOfNextLine {
        select: bool,
    },
    MoveToEndOfNextLine {
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
    MoveToNextFunction {
        count: u32,
    },
    MoveToPreviousFunction {
        count: u32,
    },
    MoveToNextClass {
        count: u32,
    },
    MoveToPreviousClass {
        count: u32,
    },
    MoveToNextArgument {
        count: u32,
    },
    MoveToPreviousArgument {
        count: u32,
    },

    InsertText(String),
    InsertNewLine,
    InsertNewLineMotion {
        count: u32,
        motion: Box<Action>,
    },
    InsertTab,

    DeleteText {
        count: usize,
    },
    Backspace {
        count: u32,
    },
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
    ChangeCurrentLine {
        count: u32,
    },

    YankMotion {
        count: u32,
        motion: Box<Action>,
    },
    YankCurrentLine {
        count: u32,
    },
    Paste {
        count: u32,
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
    SelectAround {
        kind: SelectInKind,
    },
    SelectSimilar,

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
- [x] d{motion}, dd, D (Delete with motion, Delete line, Delete to end of line - via DeleteCurrentLine)
- [x] c{motion}, cc, C (Change with motion, Change line, Change to end of line)
- [ ] y{motion}, yy, Y (Yank/Copy with motion, Yank line)
- [ ] p, P (Put/Paste after/before cursor)
- [x] u, Ctrl-r (Undo, Redo)
- [ ] . (Repeat last change)
- [ ] >, < (Indent, Unindent)

Search:
- [x] /pattern, ?pattern (Search forward/backward)
- [x] n, N (Next/Previous search result)
- [ ] *, # (Search for word under cursor forward/backward)

Visual Mode:
- [x] v, V, Ctrl-v (Character, Line, and Block visual modes)
- [ ] gv (Reselect last visual selection)

Command Mode:
- [ ] :w (Save), :q (Quit), :wq (Save and Quit)
- [ ] :%s/old/new/g (Global search and replace)
*/
