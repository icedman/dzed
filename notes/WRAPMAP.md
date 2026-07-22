# Migrating `WrapMap` to `sum_tree`

## Recommendation

Use Zed's **transform-tree representation**, but do not copy its complete `WrapMap` implementation verbatim.

Our display pipeline is much smaller: it wraps a plain `text::BufferSnapshot` at fixed terminal columns, has no preceding tab/fold display maps, no GPUI `LineWrapper`, no proportional-font measurement, and currently receives no edit patch in `WrapMap::sync`. The reusable core is the idea of representing wrapping as an ordered stream of input-preserving spans and zero-input wrap insertions in a `sum_tree::SumTree`.

A staged port will give us logarithmic point lookup immediately and leave a clean path to incremental updates later. Copying the whole upstream file would import assumptions and dependencies that do not fit this project.

## Current implementation

`src/display/wrap_map.rs` builds one `RowMapping` per buffer row:

```rust
struct RowMapping {
    display_row_start: u32,
    wrap_indices: Vec<u32>,
}
```

Every `sync`:

1. Walks every buffer row.
2. Computes fixed-width wrap columns only inside a viewport-derived range.
3. Stores the absolute display start for every row.
4. Replaces the entire `Arc<Vec<RowMapping>>`.

Point conversion then uses the vector:

- buffer point → direct row indexing plus a reverse linear scan of `wrap_indices`;
- wrap point → binary search over `display_row_start`;
- `row_count` and `max_point` → inspect the last mapping.

### Problems to fix during the migration

1. **Viewport-dependent snapshots are not globally correct.** Rows outside the selected viewport are represented as unwrapped. Consequently, `row_count`, `max_point`, point conversion, and scroll positions can change merely because the viewport moved.
2. **`set_view` does not rebuild.** It changes the range used by wrapping but does not call `sync`, so the snapshot can remain based on an old viewport.
3. **The range checks exclude boundaries.** `row > start && row < end` omits both `start` and `end`.
4. **Every sync is O(buffer rows).** Even a tiny text edit rebuilds all mappings and absolute output offsets.
5. **Wrapping is based on raw columns.** That is currently consistent with the terminal-oriented API, but tabs, Unicode display width, and word wrapping are not represented. `sum_tree` does not itself solve this; the boundary producer must eventually do so.
6. **Conversion silently falls back or clamps.** An out-of-range input point becomes `(0, 0)`, while output points can be clamped to the final mapping but retain an arbitrary column. The tree version should define clipping explicitly.

## What Zed's implementation does

The upstream implementation stores this in `WrapSnapshot`:

```rust
transforms: SumTree<Transform>
```

Each `Transform` summarizes both coordinate spaces:

```rust
struct TransformSummary {
    input: TextSummary,
    output: TextSummary,
}
```

There are two conceptual transform kinds:

- **Isomorphic span:** consumes input text and emits the same text. Its input and output summaries are equal.
- **Wrap insertion:** consumes no input and emits a newline plus optional indentation. Its input summary is zero while its output point advances by one row.

Because each subtree knows total input and output extents, a cursor can seek by either dimension:

- seek by input point to map buffer → wrapped output;
- seek by output point to map wrapped output → buffer;
- get total wrapped extent from the root summary;
- splice only affected transform ranges after edits.

This is the part worth adopting. Zed's async GPUI task management, `TabSnapshot`, pixel line wrapping, chunks, highlights, wrap indentation, and patch plumbing should not be copied until this project needs those features.

## Proposed local data model

Keep the public API initially, but replace `row_mappings` with a transform tree.

```rust
use sum_tree::SumTree;

#[derive(Clone)]
pub struct WrapSnapshot {
    buffer: BufferSnapshot,
    wrap_width: Option<u32>,
    transforms: SumTree<Transform>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Transform {
    summary: TransformSummary,
    kind: TransformKind,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TransformKind {
    #[default]
    Isomorphic,
    Wrap,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TransformSummary {
    input: Point,
    output: WrapPoint,
}
```

`TransformSummary` only needs point extents for the first port. We do not need to copy Zed's richer `TextSummary` unless later APIs require character counts, longest rows, or text chunks.

The important operation is **point composition**, not ordinary component-wise addition. When adding point `b` after point `a`:

```text
if b.row == 0: (a.row, a.column + b.column)
else:          (a.row + b.row, b.column)
```

Implement this behavior for both the input `Point` and output `WrapPoint` dimensions. Check whether the imported `Point` already supports the needed `AddAssign`; otherwise use a small helper rather than component-wise arithmetic.

### Transform constructors

```rust
Transform::isomorphic(extent)
```

- input extent = `extent`;
- output extent = the equivalent `WrapPoint`;
- kind = `Isomorphic`.

```rust
Transform::wrap()
```

- input extent = `Point::new(0, 0)`;
- output extent = `WrapPoint::new(1, 0)`;
- kind = `Wrap`.

Unlike Zed, the initial terminal version needs no stored display text because existing consumers ask for geometry, not transformed text chunks. If soft-wrap indentation is added, the output extent can become `(1, indent)` and the insertion text can be stored then.

### `sum_tree` traits

Follow the upstream implementations closely:

1. `sum_tree::Item for Transform` returns its cloned summary.
2. `ContextLessSummary for TransformSummary` composes both extents.
3. `Dimension<TransformSummary> for Point` accumulates `summary.input`.
4. `Dimension<TransformSummary> for WrapPoint` accumulates `summary.output`.
5. Implement `SeekTarget<TransformSummary, TransformSummary> for Point` if seeking a `Point` cursor requires comparing against the summary's input extent. `WrapPoint` can generally use the blanket ordered-dimension implementation if it is the cursor dimension itself; mirror upstream if type inference or combined dimensions require explicit implementations.

For conversions that need both positions at once, use:

```rust
Dimensions<WrapPoint, Point>
```

or the reverse order, but choose one ordering and use it consistently. A cursor's `start()` then provides the cumulative output and input locations at the beginning of its current transform.

## Building the tree

For the first implementation, build transforms for the **entire snapshot**, independent of the viewport.

For each buffer row:

1. Determine `line_len`.
2. For every fixed-width segment, push an isomorphic transform for the consumed columns.
3. Insert `Transform::wrap()` between segments.
4. Include the real input newline in an isomorphic transform when the buffer row is not the final row.

The newline detail is essential. A physical newline consumes input and emits a newline; a soft wrap consumes no input and emits a newline. Both move the output to the next row, but only one advances the input row.

Conceptually, for `abcdef\n` at width 3:

```text
isomorphic("abc")  input (0,3), output (0,3)
wrap                input (0,0), output (1,0)
isomorphic("def\n") input (1,0), output (1,0)
```

Merge adjacent isomorphic transforms, as upstream's `push_or_extend` does. This keeps the tree proportional to wrap boundaries rather than characters. With wrapping disabled, the whole buffer should normally be one isomorphic transform.

### How to obtain extents

Prefer snapshot range summaries if `BufferSnapshot` exposes a suitable summary API. Otherwise, the initial builder can use `row_count()` and `line_len()` and construct point extents directly. The important rule is to represent physical newlines correctly.

Before coding, verify whether `line_len(row)` excludes the newline (the current code assumes it does) and how an empty buffer's `max_point` is represented.

## Mapping algorithms

### Buffer point → `WrapPoint`

1. Clip or validate the input point against the buffer snapshot.
2. Create a cursor carrying both dimensions.
3. Seek to the input `Point` with an explicitly chosen bias.
4. Read `(output_start, input_start)` from the cursor.
5. If the current item is isomorphic, add the local input delta to `output_start`.
6. If the point lies at a soft-wrap boundary, use bias to decide whether it maps to the end of the previous display row or start of the next. For cursor movement, matching the existing behavior likely means the start of the wrapped row for the boundary column.

This replaces direct row lookup and the reverse scan of wrap indices with O(log n) seeking.

### `WrapPoint` → buffer point

1. Clip or validate against the wrapped maximum point.
2. Seek by output `WrapPoint` with both dimensions available.
3. Read `(output_start, input_start)`.
4. For an isomorphic transform, add the local output delta to `input_start`.
5. For a wrap insertion, return the insertion's input boundary. Bias decides which visual side owns the boundary.

This replaces binary search over absolute row starts and avoids storing one absolute prefix sum per source row.

### `row_count`, `max_point`, and `line_len`

- `max_point()` is the output dimension of `transforms.summary()`.
- Preserve current `row_count()` semantics carefully. The current implementation returns the number of display rows, while `max_point().row` is the zero-based last row. For a nonempty logical buffer this is normally `max_point.row + 1`.
- `line_len(display_row)` can seek to `WrapPoint::new(display_row, 0)` and determine the next output row boundary. A simpler first version may map the row start and use the wrap width plus the source line remainder, but a cursor-based `next_row_boundary` equivalent will be more robust.

Add tests before changing these semantics because empty buffers and trailing newlines are easy off-by-one cases.

## Synchronization strategy

### Phase 1: full rebuild, tree-backed queries

Keep today's method:

```rust
pub fn sync(&mut self, buffer: BufferSnapshot)
```

but rebuild a complete transform tree. This is still O(n) per sync, yet it delivers:

- correct global wrapping independent of viewport;
- O(log n) point conversion;
- cheap snapshot cloning through `SumTree`'s persistent structure;
- the right representation for later incremental edits.

At this phase, remove `scroll_y`, `screen_rows`, and `screen_cols` from `WrapMap`, or make `set_view` a no-op temporarily and then remove it from `DisplayMap`. Wrapping geometry must not depend on what is visible. Viewport virtualization, if ever needed, belongs above the canonical map or must use placeholders with exact summaries.

### Phase 2: pass edits into `sync`

Change callers to provide buffer edits, ideally as `text::Edit<Point>` or `text::Patch<Point>`:

```rust
pub fn sync(
    &mut self,
    buffer: BufferSnapshot,
    edits: &[text::Edit<Point>],
)
```

Expand each edit to whole affected rows because changing text in one row can change every soft-wrap boundary on that row. Merge overlapping/adjacent row edits, as upstream's `RowEdit` pass does.

Then rebuild only those row ranges:

1. Cursor-slice the unchanged tree prefix by old input coordinates.
2. Generate new transforms for the expanded new row range.
3. Seek past the old row range.
4. Append the structurally shared unchanged suffix.
5. Coalesce adjacent isomorphic transforms at splice boundaries.

This is the main `sum_tree` payoff: update cost becomes approximately O(changed text + changed wraps + log n), with unchanged tree nodes shared by snapshots.

### Phase 3: optional patch output

Only add `WrapPatch` if rendering/layout can use it to invalidate changed display rows. The full Zed pipeline computes output edits from old and new transform trees. That is useful, but not required merely to replace `row_mappings`.

### Phase 4: better wrap boundaries

Separate the tree representation from the boundary algorithm. Introduce a small interface/function that yields wrap columns for a row. Initially it can preserve current fixed-column wrapping. Later it can account for:

- tab expansion;
- Unicode terminal cell width (`unicode-width` or an existing project facility);
- grapheme boundaries;
- word boundaries;
- continuation indentation.

Never split in the middle of a UTF-8 byte sequence or a terminal grapheme. Confirm what `Point.column` measures before selecting the boundary implementation; Zed's points and terminal screen cells are not automatically equivalent.

## What not to copy yet

Do not initially copy these upstream pieces:

- `gpui::{LineWrapper, Pixels, Font, Task}` and background entity management;
- `TabSnapshot`/`TabEdit` integration;
- chunk and highlight iterators;
- `display_text` unless consumers need transformed text;
- `soft_wrap_indent` and proportional-font measurement;
- interpolation while an async rewrap is pending;
- `WrapPatch` composition;
- row metadata tied to folds/multi-buffers.

They solve real Zed editor requirements but would obscure the small tree migration and pull the implementation away from this terminal app's needs.

## Suggested implementation order

1. **Add characterization tests for the current public API.** Cover empty buffer, empty lines, trailing newline, width `None`, width `0`, exact-width lines, multiple wraps, boundary columns, last point, and round trips.
2. **Fix/define clipping and boundary bias.** Record expected behavior in tests rather than retaining accidental fallback-to-zero behavior.
3. **Introduce `Transform`, `TransformSummary`, and trait implementations** in `src/display/wrap_map.rs`.
4. **Write a full-snapshot transform builder** with fixed-column boundaries.
5. **Replace `row_mappings` in `WrapSnapshot`** and port `max_point`, `row_count`, and both point conversions.
6. **Port `line_len`** using cursor row boundaries and add exhaustive small examples.
7. **Remove viewport-dependent wrapping state** and adjust `src/display/display_map.rs` call sites.
8. **Run unit tests plus randomized round-trip/property tests.** For every valid buffer point, require `from_wrap_point(to_wrap_point(p)) == p` except at explicitly biased ambiguous boundaries. Test the reverse direction for valid visual points as well.
9. **Only then add edit-aware splicing.** First expose edits at the caller boundary, then port the row-range slice/append approach from upstream.
10. **Benchmark** full rebuild and incremental edit workloads before adding async/background complexity.

## Invariants worth copying

Upstream's `check_invariants` idea is valuable even if its exact checks do not transfer. In debug builds/tests verify:

- tree input summary equals the buffer's complete point extent;
- wrapping disabled implies input extent equals output extent;
- wrap transforms consume zero input and advance output by exactly one row (plus any defined indent);
- isomorphic transforms have equal input/output extents;
- no two adjacent isomorphic transforms remain unmerged;
- the tree does not begin or end with an invalid synthetic wrap;
- `max_point` equals the tree's output summary;
- point conversions are monotonic;
- all valid source points round-trip under the selected bias policy.

## Expected result

After Phase 1, the representation is globally correct and queries are logarithmic, though synchronization remains a full rebuild. After Phase 2, ordinary edits can reuse unchanged tree prefixes/suffixes and rewrap only affected physical rows. This captures the useful architecture of Zed's `WrapMap` without importing its UI stack or prematurely reproducing its entire display-map pipeline.

The central design rule is: **store local input/output transforms and let `sum_tree` maintain prefix sums; do not store absolute `display_row_start` values per row.**
