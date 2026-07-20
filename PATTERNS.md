# Oniguruma (Onig) regex patterns with syntect

syntect uses the Oniguruma (Onig) regular expression engine via the `onig` crate, the same engine Sublime Text grammars target. This means patterns in `.sublime-syntax` and `.tmLanguage` files use Onig syntax and features like look-around, named groups, Unicode properties, etc.

This cheatsheet shows practical Onig constructs and idioms you can use when writing patterns for syntax highlighting, and how to test them in Rust.

## Quick reference: flags and basics

Inline flags (toggle within a pattern):
- `(?i)` case-insensitive, `(?-i)` turn off
- `(?m)` multi-line: `^` and `$` match at line boundaries
- `(?s)` single-line (dotall): `.` matches newlines
- `(?x)` extended: ignore whitespace and allow comments in pattern
- `(?U)` ungreedy by default (quantifiers lazy unless suffixed by `?`)
- `(?u)` Unicode mode (word chars/boundaries respect Unicode)

Anchors:
- `^` start of line (with `m`), `\A` start of string
- `$` end of line (with `m`), `\z` end of string, `\Z` end before final newline
- `\G` end of previous match (useful for iterative lexing)

Character classes:
- `\w`, `\W`, `\d`, `\D`, `\s`, `\S` (ASCII unless `(?u)`)
- Unicode properties: `\p{L}`, `\p{Ll}`, `\p{Lu}`, `\p{N}`, `\p{XID_Start}`, `\p{XID_Continue}`, etc.
- POSIX classes inside `[]`: `[:alpha:]`, `[:alnum:]`, `[:space:]`, ...

Groups and alternation:
- Capturing: `( ... )`
- Non-capturing: `(?: ... )`
- Named capture: `(?<name> ... )` and backref with `\k<name>` or `\k'name'`
- Alternation: `a|b|c`

Quantifiers and backtracking control:
- Greedy: `*`, `+`, `?`, `{m,n}`
- Lazy: `*?`, `+?`, `??`, `{m,n}?`
- Possessive (no backtrack): `*+`, `++`, `?+`, `{m,n}+` (supported by Onig)
- Atomic group (no backtrack): `(?> ... )`

Look-around:
- Lookahead: `(?= ... )`, `(?! ... )`
- Lookbehind: `(?<= ... )`, `(?<! ... )` (variable-length is supported with constraints; prefer fixed-length when possible)

Word boundaries:
- `\b`, `\B` (ASCII by default; add `(?u)` for Unicode semantics)

## Common idioms for grammars

- Identifier start/continue (Unicode-aware):
  - Start: `(?u)[\p{XID_Start}_]`
  - Continue: `(?u)[\p{XID_Continue}_]`
  - Whole identifier: `(?u)[\p{XID_Start}_][\p{XID_Continue}_]*`

- Function definition (simple):
  - `(?x) ^ \s* (?:pub\s+)? fn \s+ (?<name>[A-Za-z_][A-Za-z0-9_]*) \s* \(`

- Line comment (C/C++/Rust style):
  - `//.*$`

- Block comment (non-nested):
  - `/\*[^*]*\*+(?:[^/*][^*]*\*+)*/`

- Strings with escapes (double-quoted, simple):
  - `" (?: \\ . | [^"\\] )*? "`

- Triple-quoted (Python):
  - `(?s)""".*?"""`

- Heredoc start (shell-like):
  - `^\s*<<-?\s*(?<tag>[A-Z_][A-Z0-9_]*)\b`

- Numbers (hex/bin/dec, simple):
  - `(?ix)
    (?: 0x[0-9A-F_]+ | 0b[01_]+ | \d(?:[_\d]*\d)? (?: \.[_\d]*\d )? (?: [eE][+-]?\d+ )? )`

- Matching balanced parentheses (limited, via tempered dot):
  - `\((?:[^()]+|\([^()]*\))*\)`

## Using Onig in Rust directly

```rust
use onig::Regex;

fn main() {
    let text = "fn greet(name: &str) { println!(\"hi\") }";
    let re = Regex::new(r"(?x)
        \b fn \s+ (?<name>[A-Za-z_][A-Za-z0-9_]*) \s* \(
    ").unwrap();

    for (i, caps) in re.captures_iter(text).enumerate() {
        let m = caps.at(0).unwrap();
        let name = caps.name("name").unwrap();
        println!("{}: {:?} function name = {:?}", i, m, name);
    }
}
```

## Using patterns in syntect grammars (Sublime-style)

Sublime grammars define contexts and patterns with Onig regex. Example (YAML for `.sublime-syntax`):

```yaml
name: Mini Rust
scope: source.rust
contexts:
  main:
    - match: (?x) \b (?:pub\s+)? fn \s+ (?<name>[A-Za-z_][A-Za-z0-9_]*) \b
      captures:
        0: meta.function.rust
        name: entity.name.function.rust
    - match: //.*$
      scope: comment.line.double-slash.rust
    - match: '"'
      push: string

  string:
    - meta_scope: string.quoted.double.rust
    - match: '\\.'
      scope: constant.character.escape.rust
    - match: '"'
      pop: true
```

Load it with syntect:

```rust
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use syntect::easy::HighlightFile;

// Prebuilt syntaxes
let ss = SyntaxSet::load_defaults_newlines();

// Or build your own set from paths
// let mut builder = SyntaxSetBuilder::new();
// builder.add_from_folder("syntaxes", true).unwrap();
// let ss = builder.build();
```

## Tips for robust patterns

- Prefer Unicode-aware classes with `(?u)` when matching identifiers in non-ASCII files.
- Use lazy quantifiers `*?` and explicit character classes to avoid catastrophic backtracking.
- Use possessive quantifiers `++` and atomic groups `(?>...)` to fence off large regions when feasible.
- Keep lookbehind fixed-width if possible; some engines and tools struggle with variable-length lookbehind despite Onig support.
- Anchor with `^`/`$` (with `(?m)`) when patterns are line-local; this improves performance.
- Test incrementally with small inputs; large ambiguous alternations can be slow.

## Testing and debugging

- Rust `onig` crate: minimal harness to iterate matches and print capture spans.
- Sublime Text’s "Show Scope Name" helps verify scopes from grammar patterns.
- Online testers for Ruby regex can be a decent proxy (Ruby uses Onigmo, a fork of Oniguruma), but minor differences exist—always verify in-code.

## References

- syntect documentation: https://github.com/trishume/syntect
- Oniguruma syntax reference: https://github.com/kkos/oniguruma/blob/master/doc/RE
- Sublime Text `.sublime-syntax` docs: https://www.sublimetext.com/docs/syntax.html
- Onig crate: https://crates.io/crates/onig
