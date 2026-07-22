pub mod grammars;
pub mod tree_sitter;

pub use tree_sitter::{
    ParseError, QueryCapture, ScopeInfo, SyntaxNode, SyntaxTree, TreeSitterParser,
};
