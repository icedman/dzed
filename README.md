# DZed (Demaked Zed)

DZed is an experimental terminal text editor inspired by **Zed's editing architecture** and **Vim's modal editing model**.

The goal is not to reproduce either editor exactly. DZed explores what a fast, compact, Vim-oriented terminal editor can look like when built on top of Zed's core text infrastructure, including its rope, CRDT buffer, anchors, sum trees, and incremental display mapping.

DZed is currently a proof of concept and an active learning project rather than a production-ready editor.

![DZed screenshot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%202026-02-02%2021-56-19.png)

## Project Goals

- Bring Zed's high-performance text and buffer primitives to a terminal interface.
- Build a Vim-inspired modal editing experience without depending on Vim itself.
- Experiment with selections, multiple cursors, text objects, display maps, and asynchronous editor services.
- Keep the codebase small enough to understand, modify, and use as a platform for editor experiments.
- Gradually improve Vim compatibility while retaining the flexibility to adopt ideas from Zed and other modern editors.

## Features So Far

### Zed-Based Text Engine

- CRDT-backed text buffers and stable anchors.
- Rope-based text storage and editing.
- Sum-tree indexing through Zed's core crates.
- Incremental buffer snapshots used by rendering, wrapping, and highlighting.
- Multiple open buffers with buffer switching.

### Vim-Inspired Modal Editing

- Normal, Insert, Command, Visual, Visual Line, and Visual Block modes.
- Vim-style motions across characters, words, lines, paragraphs, and documents.
- Motion counts and composed operator-motion commands.
- Character search motions and repeatable multi-key command sequences.
- Insert, append, open-line, change, delete, undo, and redo operations.
- Mode-aware cursor behavior and selection extension.
- Macro recording (`q{c}` to record to register `{c}`, `q` to stop recording) and replaying (`@{c}` with support for count prefix to repeat execution).

### Selections and Text Objects

- Characterwise, linewise, and blockwise visual selections.
- Selection synchronization for Visual Line and Visual Block modes.
- Word-based text objects, including inside and around selections.
- Motion-based delete, change, and yank operations.
- Multiple cursors and selection of similar text occurrences.
- Orientation-independent selection range handling.
- Text extraction from individual selections and combined multi-selections.

### Clipboard and Yank/Paste

- Internal editor clipboard owned by the `Editor`.
- Characterwise, linewise, and blockwise clipboard metadata.
- Motion-based and linewise yank support.
- Characterwise and linewise paste behavior.
- Paste counts and cursor restoration after yank operations.
- Named register support (`"{register}y` to yank to register, `"{register}p` to paste).

### Display and Terminal UI

- Terminal interface built with `crossterm`.
- Soft wrapping through custom `WrapMap` and `DisplayMap` layers.
- Logical buffer-point to display-point conversion.
- Scrolling that follows the active cursor.
- Optional line-number gutter.
- Syntax highlighting powered by `syntect` and AST-based syntax folding/parsing using `tree-sitter` (supporting Rust, Go, Python, HTML, Markdown, etc.).
- Search-match and selection highlighting.
- Dedicated buffer, status-bar, command-line, and cursor renderers.
- Mode-specific terminal cursor styles.
- Block code folding.
- Bracketed paste and mouse-capture support.

### Search and Commands

- Plain-text and regular-expression search.
- Forward and backward match navigation.
- Search and command history.
- Bounded cross-row match searches used by multi-cursor selection.
- Basic command-line operations for buffers, themes, wrapping, syntax highlighting, line numbers, and line navigation.

### Background Work

- Asynchronous syntax-highlighting tasks.
- Asynchronous wrapping tasks.
- Asynchronous tree-sitter tasks.
- Task identifiers prevent stale background results from replacing newer editor state.

### Extensible Input Architecture

- Key-event handling is separated from keymap definitions.
- Normal, Insert, and pending multi-key command maps are represented independently.
- Runtime key-binding APIs provide a foundation for future configurable keymaps.
- Actions are resolved separately from document mutation and rendering.

## Getting Started

### Build

```sh
cargo build -p test_zed
```

### Run

```sh
cargo run -p test_zed -- <path-to-file>
```

Multiple paths may be supplied to open more than one buffer.

### Test

```sh
cargo test -p test_zed
```

## Project Status

DZed is under active development. Editing behavior, internal APIs, and file organization may change frequently. Important areas still being developed include broader Vim parity, richer text objects, complete clipboard semantics, persistence and save commands, configurable keymaps, and more robust terminal interaction.

## AI Disclaimer

This project has been built largely through AI-assisted development using Gemini and GPT models. I wrote the initial prototype while studying Zed's core components—particularly its sum tree, rope, text, and editor implementations. AI has since become an integral part of implementation, refactoring, debugging, and experimentation.

## Contributing

Contributions are welcome, especially those that improve Vim-style editing, deepen integration with Zed's crates, add tests for editor behavior, or simplify the architecture without hiding how it works.

## Support Me

[![Buy Me A Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/icedman)

If you find the project useful and would like to support continued development, contributions help cover development and AI-assisted tooling costs.

## Visual Progress

![DZed screenshot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%202026-02-02%2021-56-19.png)
![DZed screenshot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%202026-07-31%2007-32-03.png)
![DZed screenshot](https://raw.githubusercontent.com/icedman/dzed/refs/heads/main/screenshots/Screenshot%20from%2022026-08-01%2008-15-38.png)
