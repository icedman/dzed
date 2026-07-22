use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use text::BufferSnapshot;
use tree_sitter::{
    LanguageError, Node, Parser, Point, Query, QueryCursor, QueryError, StreamingIterator, Tree,
};

use super::grammars::Grammar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    pub kind: String,
    pub named: bool,
    pub byte_range: Range<usize>,
    pub start_position: Point,
    pub end_position: Point,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryCapture {
    pub name: String,
    pub node: SyntaxNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeInfo {
    pub kind: String,
    pub name: Option<String>,
    pub byte_range: Range<usize>,
}

#[derive(Clone)]
pub struct SyntaxTree {
    grammar: Grammar,
    tree: Tree,
    scope_cache: Arc<Mutex<HashMap<usize, Vec<SyntaxNode>>>>,
}

impl SyntaxTree {
    pub fn grammar(&self) -> Grammar {
        self.grammar
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    pub fn root_kind(&self) -> &str {
        self.tree.root_node().kind()
    }

    pub fn node_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, false).map(Self::node_info)
    }

    pub fn named_node_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, true).map(Self::node_info)
    }

    pub fn parent_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, true)
            .and_then(|node| node.parent())
            .map(Self::node_info)
    }

    pub fn first_named_child_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, true)
            .and_then(|node| node.named_child(0))
            .map(Self::node_info)
    }

    pub fn next_named_sibling_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, true)
            .and_then(|node| node.next_named_sibling())
            .map(Self::node_info)
    }

    pub fn previous_named_sibling_at_byte(&self, byte: usize) -> Option<SyntaxNode> {
        self.descendant_at_byte(byte, true)
            .and_then(|node| node.prev_named_sibling())
            .map(Self::node_info)
    }

    pub fn scope_path_at_byte(&self, byte: usize) -> Vec<SyntaxNode> {
        if let Some(cached) = self.scope_cache.lock().unwrap().get(&byte).cloned() {
            return cached;
        }

        let Some(mut node) = self.descendant_at_byte(byte, true) else {
            return Vec::new();
        };
        let mut path = Vec::new();
        loop {
            path.push(Self::node_info(node));
            let Some(parent) = node.parent() else {
                break;
            };
            node = parent;
        }
        path.reverse();
        self.scope_cache.lock().unwrap().insert(byte, path.clone());
        path
    }

    pub fn current_scope(&self, source: &BufferSnapshot, byte: usize) -> Option<ScopeInfo> {
        const SCOPE_KINDS: &[&str] = &[
            // Rust
            "function_item",
            "impl_item",
            "trait_item",
            "mod_item",
            "struct_item",
            "enum_item",
            "closure_expression",
            // Bash, C, Go, Python, JavaScript, and TypeScript
            "function_definition",
            "function_declaration",
            "method_declaration",
            "method_definition",
            "class_definition",
            "class_declaration",
            "interface_declaration",
            "type_alias_declaration",
            "arrow_function",
            // HTML and CSS containers
            "element",
            "rule_set",
        ];

        let mut node = self.descendant_at_byte(byte, true)?;
        loop {
            if SCOPE_KINDS.contains(&node.kind()) {
                let name = node
                    .child_by_field_name("name")
                    .map(|name| Self::text_for_node(source, name));
                return Some(ScopeInfo {
                    kind: node.kind().to_string(),
                    name,
                    byte_range: node.byte_range(),
                });
            }
            node = node.parent()?;
        }
    }

    pub fn query(
        &self,
        source: &BufferSnapshot,
        query_source: &str,
    ) -> Result<Vec<QueryCapture>, QueryError> {
        let query = Query::new(&self.grammar.language(), query_source)?;
        let source_text: String = source
            .as_rope()
            .chunks_in_range(0..source.as_rope().len())
            .collect();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, self.tree.root_node(), source_text.as_bytes());
        let capture_names = query.capture_names();
        let mut captures = Vec::new();

        while let Some(query_match) = matches.next() {
            for capture in query_match.captures {
                captures.push(QueryCapture {
                    name: capture_names[capture.index as usize].to_string(),
                    node: Self::node_info(capture.node),
                });
            }
        }

        Ok(captures)
    }

    fn descendant_at_byte(&self, byte: usize, named: bool) -> Option<Node<'_>> {
        let root = self.tree.root_node();
        if root.end_byte() == 0 {
            return Some(root);
        }
        let start = byte.min(root.end_byte().saturating_sub(1));
        let end = start.saturating_add(1).min(root.end_byte());
        if named {
            root.named_descendant_for_byte_range(start, end)
        } else {
            root.descendant_for_byte_range(start, end)
        }
    }

    fn node_info(node: Node<'_>) -> SyntaxNode {
        SyntaxNode {
            kind: node.kind().to_string(),
            named: node.is_named(),
            byte_range: node.byte_range(),
            start_position: node.start_position(),
            end_position: node.end_position(),
        }
    }

    fn text_for_node(source: &BufferSnapshot, node: Node<'_>) -> String {
        source
            .as_rope()
            .chunks_in_range(node.byte_range())
            .collect()
    }
}

impl fmt::Debug for SyntaxTree {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SyntaxTree")
            .field("grammar", &self.grammar)
            .field("root_kind", &self.root_kind())
            .field("has_error", &self.tree.root_node().has_error())
            .finish()
    }
}

#[derive(Debug)]
pub enum ParseError {
    IncompatibleLanguage(LanguageError),
    Cancelled,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleLanguage(error) => write!(formatter, "incompatible grammar: {error}"),
            Self::Cancelled => formatter.write_str("tree-sitter parsing was cancelled"),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct TreeSitterParser {
    parser: Parser,
    grammar: Grammar,
}

impl TreeSitterParser {
    pub fn new(grammar: Grammar) -> Result<Self, ParseError> {
        let mut parser = Parser::new();
        parser
            .set_language(&grammar.language())
            .map_err(ParseError::IncompatibleLanguage)?;
        Ok(Self { parser, grammar })
    }

    pub fn parse(
        &mut self,
        snapshot: &BufferSnapshot,
        old_tree: Option<&SyntaxTree>,
    ) -> Result<SyntaxTree, ParseError> {
        let rope = snapshot.as_rope();
        let mut chunks = rope.chunks_in_range(0..rope.len());
        let old_tree = old_tree
            .filter(|tree| tree.grammar == self.grammar)
            .map(|tree| tree.tree());

        let tree = self
            .parser
            .parse_with_options(
                &mut move |offset, _| {
                    chunks.seek(offset);
                    chunks.next().unwrap_or("").as_bytes()
                },
                old_tree,
                None,
            )
            .ok_or(ParseError::Cancelled)?;

        Ok(SyntaxTree {
            grammar: self.grammar,
            tree,
            scope_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clock::ReplicaId;
    use text::{Buffer, BufferId};

    #[test]
    fn parses_a_buffer_snapshot_without_flattening_it() {
        let source = "fn main() { let value = 42; }";
        let buffer = Buffer::new(ReplicaId::LOCAL, BufferId::new(1).unwrap(), source);
        let mut parser = TreeSitterParser::new(Grammar::Rust).unwrap();
        let syntax = parser.parse(&buffer.snapshot(), None).unwrap();

        assert_eq!(syntax.root_kind(), "source_file");
        assert!(!syntax.tree().root_node().has_error());

        let value_offset = source.find("value").unwrap();
        assert_eq!(
            syntax.named_node_at_byte(value_offset).unwrap().kind,
            "identifier"
        );
        assert!(syntax.parent_at_byte(value_offset).is_some());

        let scope = syntax
            .current_scope(buffer.snapshot(), value_offset)
            .expect("cursor should be inside a function scope");
        assert_eq!(scope.kind, "function_item");
        assert_eq!(scope.name.as_deref(), Some("main"));

        let first_path = syntax.scope_path_at_byte(value_offset);
        let cached_path = syntax.scope_path_at_byte(value_offset);
        assert_eq!(first_path, cached_path);
        assert_eq!(first_path.last().unwrap().kind, "identifier");

        let captures = syntax
            .query(
                buffer.snapshot(),
                "(function_item name: (identifier) @function.name)",
            )
            .unwrap();
        assert_eq!(captures.len(), 1);
        assert_eq!(captures[0].name, "function.name");
        assert_eq!(captures[0].node.kind, "identifier");
    }
}
