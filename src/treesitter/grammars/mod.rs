use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Bash,
    C,
    Css,
    Go,
    Html,
    JavaScript,
    Json,
    Python,
    Rust,
    TypeScript,
    Tsx,
    Zig,
    Java,
    CSharp,
    Cpp,
    PowerShell,
    Php,
    Lua,
    Assembly,
    Ruby,
    Dart,
    Swift,
    R,
    Groovy,
}

impl Grammar {
    pub const ALL: [Self; 24] = [
        Self::Bash,
        Self::C,
        Self::Css,
        Self::Go,
        Self::Html,
        Self::JavaScript,
        Self::Json,
        Self::Python,
        Self::Rust,
        Self::TypeScript,
        Self::Tsx,
        Self::Zig,
        Self::Java,
        Self::CSharp,
        Self::Cpp,
        Self::PowerShell,
        Self::Php,
        Self::Lua,
        Self::Assembly,
        Self::Ruby,
        Self::Dart,
        Self::Swift,
        Self::R,
        Self::Groovy,
    ];

    pub fn from_path(path: &str) -> Option<Self> {
        let path = Path::new(path);
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        match file_name.as_str() {
            ".bashrc" | ".bash_profile" | ".profile" | "bashrc" | "bash_profile" => {
                return Some(Self::Bash);
            }
            _ => {}
        }

        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())?
            .to_ascii_lowercase();

        match extension.as_str() {
            "sh" | "bash" => Some(Self::Bash),
            "c" | "h" => Some(Self::C),
            "css" => Some(Self::Css),
            "go" => Some(Self::Go),
            "html" | "htm" => Some(Self::Html),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "json" | "jsonc" => Some(Self::Json),
            "py" | "pyi" | "pyw" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "zig" => Some(Self::Zig),
            "java" => Some(Self::Java),
            "cs" => Some(Self::CSharp),
            "cpp" | "cc" | "cxx" | "hpp" | "h++" => Some(Self::Cpp),
            "ps1" | "psm1" | "psd1" => Some(Self::PowerShell),
            "php" | "phtml" => Some(Self::Php),
            "lua" => Some(Self::Lua),
            "asm" | "s" => Some(Self::Assembly),
            "rb" | "ru" => Some(Self::Ruby),
            "dart" => Some(Self::Dart),
            "swift" => Some(Self::Swift),
            "r" | "rmd" => Some(Self::R),
            "groovy" | "gvy" | "gy" | "gsh" => Some(Self::Groovy),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bash => "Bash",
            Self::C => "C",
            Self::Css => "CSS",
            Self::Go => "Go",
            Self::Html => "HTML",
            Self::JavaScript => "JavaScript",
            Self::Json => "JSON",
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::Zig => "Zig",
            Self::Java => "Java",
            Self::CSharp => "C#",
            Self::Cpp => "C++",
            Self::PowerShell => "PowerShell",
            Self::Php => "PHP",
            Self::Lua => "Lua",
            Self::Assembly => "Assembly",
            Self::Ruby => "Ruby",
            Self::Dart => "Dart",
            Self::Swift => "Swift",
            Self::R => "R",
            Self::Groovy => "Groovy",
        }
    }

    pub fn language(self) -> tree_sitter::Language {
        match self {
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::PowerShell => tree_sitter_powershell::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            Self::Assembly => tree_sitter_asm::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Dart => tree_sitter_dart::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::R => tree_sitter_r::LANGUAGE.into(),
            Self::Groovy => tree_sitter_groovy::LANGUAGE.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_grammars_from_file_names() {
        let cases = [
            ("src/main.rs", Grammar::Rust),
            ("script.sh", Grammar::Bash),
            ("/home/user/.bashrc", Grammar::Bash),
            ("main.c", Grammar::C),
            ("style.CSS", Grammar::Css),
            ("main.go", Grammar::Go),
            ("index.html", Grammar::Html),
            ("app.jsx", Grammar::JavaScript),
            ("settings.json", Grammar::Json),
            ("types.pyi", Grammar::Python),
            ("app.ts", Grammar::TypeScript),
            ("component.tsx", Grammar::Tsx),
        ];

        for (path, expected) in cases {
            assert_eq!(Grammar::from_path(path), Some(expected), "{path}");
        }
        assert_eq!(Grammar::from_path("README"), None);
    }

    #[test]
    fn every_built_in_grammar_loads() {
        for grammar in Grammar::ALL {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&grammar.language())
                .unwrap_or_else(|error| {
                    panic!("{} grammar failed to load: {error}", grammar.name())
                });
        }
    }
}
