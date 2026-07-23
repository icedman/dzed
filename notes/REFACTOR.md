# Refactoring Recommendations & Rust Anti-Patterns

A code review of the `dzd` editor codebase reveals several opportunities to align with modern, idiomatic Rust design patterns. This document highlights key anti-patterns currently in the project and details recommendations for restructuring.

---

## 1. Document Swap Anti-Pattern (Duplicate Borrow Bypass)
### Location
* `src/editor.rs` in `apply_active_action` and `apply_command_action`

### Problem
To bypass the Rust borrow checker (since calling `Document::apply_action` requires mutable access to the document while borrowing `self` as the editor), the code uses `std::mem::replace` to swap in a dummy document:
```rust
let mut document = std::mem::replace(
    &mut self.buffer_manager.buffers[active_idx].doc,
    Document::new("").unwrap(),
);
document.apply_action(action, self);
self.buffer_manager.buffers[active_idx].doc = document;
```

### Recommendation
* **Decouple Data & Execution**: Instead of passing the whole `&mut Editor` to `Document`, pass only the specific data layers the action requires (e.g., `&Theme`, `&Clipboard`, or input state).
* **Command Pattern**: Turn editor actions into standalone command structs or functions that accept separate, disjoint borrows:
  ```rust
  fn execute_action(action: &Action, doc: &mut Document, clipboard: &mut Clipboard, mode: &mut Mode)
  ```

---

## 2. Hard Unwraps & Lack of Error Boundaries
### Location
* `src/main.rs`, `src/ui/mod.rs`

### Problem
Frequent uses of `.unwrap()` on critical runtime operations:
* Getting terminal sizes: `crossterm::terminal::size().unwrap()`
* Executing terminal commands: `execute!(stdout, MoveTo(...)).unwrap()`
* Spawning background tasks.
If a user resizes their terminal too rapidly or runs the editor in an unsupported TTY, these panics will crash the process instantly.

### Recommendation
* **Propagate Errors**: bubble up terminal and rendering errors using the `Result` type (`?` operator).
* **Graceful Degradation**: Wrap rendering functions in recovery boundaries. If terminal querying fails, fallback to standard sizing default values.

---

## 3. Monolithic Main Loop Responsibilities
### Location
* `src/main.rs`

### Problem
The `main` function coordinates:
1. Raw terminal setup / teardown.
2. Background thread parsing, wrapping, and syntax highlight message polling.
3. Cursor scroll math (`scroll_to_cursor`).
4. Sizing computations for word-wrap.
5. Keyboard/Mouse event looping and keymapping.

### Recommendation
* **Introduce an `App` Struct**: Move event loop and application state out of `main.rs` and encapsulate it in a `struct App`.
* **Separate Concerns**:
  * **`BackgroundManager`**: Handles polling background threads and applying wrap/syntax snapshot updates.
  * **`InputHandler`**: Manages event polling, pastes, and key combos.

---

## 4. Inefficient Layout Tree Computations
### Location
* `src/ui/layout.rs`

### Problem
Layout computations are calculated dynamically inside the frame drawing cycles:
```rust
let computed_layouts = ui.layout.compute_layout(parent_rect);
```
Since the layout tree is static between terminal resizing events, re-computing it on every single frame refresh wastes CPU cycles.

### Recommendation
* **Cache Layout States**: Store the computed pane dimensions in the `Ui` struct. Only recompute them when the window receives a `crossterm::event::Event::Resize` event.

---

## 5. Global/Shared Mutable State
### Location
* `src/editor.rs` (`latest_hl_task_id`, `latest_wrap_task_id`, etc.)

### Problem
Shared mutable states inside buffer structs use atomic references like `Arc<AtomicU64>` to coordinate tasks with background worker threads. This complicates the memory model and ownership tracking of editor buffers.

### Recommendation
* **Use Message Passing**: Track active task IDs inside the background worker system itself, returning completed payload metadata matching the request task ID on the completion channel, rather than sharing atomic references between threads.
