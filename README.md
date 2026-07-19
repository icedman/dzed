DZED is a proof of concept editor - Demaked Zed which brings Zed's crates (text editor, sum_tree, etc.. to the terminal)

Input and rendering handled by crossterm. Syntax highlighting by syntect. The editor emulates vim mode. See the limited features/action below

```sh
cargo build
./target/debug/test_zed <your code file>
```

![Screen Shot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%202026-02-02%2021-56-19.png)

```js
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
```
