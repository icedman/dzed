# DZED (Demaked Zed)

DZED is a proof-of-concept terminal text editor built by bringing Zed's core crates (`text`, `sum_tree`, `rope`, etc.) to the terminal. It combines the advanced text manipulation logic of Zed with a lightweight `crossterm` interface and `syntect` for syntax highlighting.

![Screen Shot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%202026-02-02%2021-56-19.png)

## Core Features

- **Zed-Powered Logic**: Uses Zed's high-performance CRDT-based buffer and sum-tree indexing.
- **Vim Emulation**: Robust Vim-style modal editing.
- **Soft Wrapping**: Full support for logical-to-screen coordinate mapping (WrapMap/DisplayMap).
- **Syntax Highlighting**: Fast rendering using `syntect`.
- **Bracketed Paste**: Efficiently handle large pastes from the clipboard.
- **Modern Terminal UI**: Mode-specific cursors (Bar in Insert, Block in Normal) and status bar.

## Implemented Actions

### Navigation
- **Basic**: `h`, `j`, `k`, `l` (Left, Down, Up, Right)
- **Word**: `w`, `b` (Start of word), `e`, `ge` (End of word)
- **Line**: `0`, `$`, `^` (Start, End, first non-blank)
- **Document**: `gg`, `G` (Start, End of file)
- **Paragraph**: `{`, `}` (Previous, Next empty line)
- **Jump**: `:{N}` jump to specific line number.
- **Count Support**: Most motions support numeric prefixes (e.g., `5w`, `10j`).

### Editing
- **Modes**: Normal, Insert, Visual, Visual Line, Command.
- **Operators**: `d{motion}` (e.g., `dw`, `df)`, `d$`) for flexible deletion.
- **Shorthands**: `x` (delete char), `dd` (delete line).
- **History**: `u` (Undo), `Ctrl-r` (Redo) with count support.
- **Formatting**: `>` and `<` for indentation.

## Getting Started

```sh
# Build the project
cargo build

# Run the editor
./target/debug/test_zed <path_to_file>
```

## Contributing
This is a proof-of-concept. Contributions that further integrate Zed's crates or improve Vim parity are welcome.
