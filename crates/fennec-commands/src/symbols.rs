use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ItemTrait, ItemType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Function,
    Struct,
    Enum,
    Trait,
    Type,
    Const,
    Module,
    Impl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub path: PathBuf,
    pub line: usize,
    pub visibility: Visibility,
    pub doc_comment: Option<String>,
}

impl Symbol {
    pub fn new(
        name: String,
        symbol_type: SymbolType,
        path: PathBuf,
        line: usize,
        visibility: Visibility,
    ) -> Self {
        Self {
            name,
            symbol_type,
            path,
            line,
            visibility,
            doc_comment: None,
        }
    }

    pub fn with_doc(mut self, doc: String) -> Self {
        self.doc_comment = Some(doc);
        self
    }
}

/// Visitor for extracting symbols from Rust AST
struct SymbolVisitor {
    symbols: Vec<Symbol>,
    file_path: PathBuf,
    current_module: Vec<String>,
}

impl SymbolVisitor {
    fn new(file_path: PathBuf) -> Self {
        Self {
            symbols: Vec::new(),
            file_path,
            current_module: Vec::new(),
        }
    }

    fn extract_visibility(vis: &syn::Visibility) -> Visibility {
        match vis {
            syn::Visibility::Public(_) => Visibility::Public,
            syn::Visibility::Restricted(r) => {
                if let Some(seg) = r.path.segments.first() {
                    if seg.ident == "crate" {
                        Visibility::Crate
                    } else if seg.ident == "super" {
                        Visibility::Super
                    } else {
                        Visibility::Private
                    }
                } else {
                    Visibility::Private
                }
            }
            syn::Visibility::Inherited => Visibility::Private,
        }
    }

    fn get_line_number(span: proc_macro2::Span) -> usize {
        // Extract line number from span (requires proc-macro2 span-locations feature)
        span.start().line
    }
}

impl<'ast> Visit<'ast> for SymbolVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.sig.ident.span());

        let symbol = Symbol::new(
            node.sig.ident.to_string(),
            SymbolType::Function,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);
        visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.ident.span());

        let symbol = Symbol::new(
            node.ident.to_string(),
            SymbolType::Struct,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);
        visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.ident.span());

        let symbol = Symbol::new(
            node.ident.to_string(),
            SymbolType::Enum,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);
        visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.ident.span());

        let symbol = Symbol::new(
            node.ident.to_string(),
            SymbolType::Trait,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);
        visit::visit_item_trait(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast ItemType) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.ident.span());

        let symbol = Symbol::new(
            node.ident.to_string(),
            SymbolType::Type,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);
        visit::visit_item_type(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let visibility = Self::extract_visibility(&node.vis);
        let line = Self::get_line_number(node.ident.span());

        let symbol = Symbol::new(
            node.ident.to_string(),
            SymbolType::Module,
            self.file_path.clone(),
            line,
            visibility,
        );

        self.symbols.push(symbol);

        // Track module nesting
        self.current_module.push(node.ident.to_string());
        visit::visit_item_mod(self, node);
        self.current_module.pop();
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Extract impl block information
        if let Some((_, trait_path, _)) = &node.trait_ {
            // This is a trait implementation
            if let Some(segment) = trait_path.segments.last() {
                let line = Self::get_line_number(segment.ident.span());
                let symbol = Symbol::new(
                    format!("impl {}", segment.ident),
                    SymbolType::Impl,
                    self.file_path.clone(),
                    line,
                    Visibility::Private, // Impls don't have visibility
                );
                self.symbols.push(symbol);
            }
        }

        visit::visit_item_impl(self, node);
    }
}

/// Extract symbols from Rust source code
pub fn extract_symbols(file_path: &Path, content: &str) -> Result<Vec<Symbol>, String> {
    let syntax_tree =
        syn::parse_file(content).map_err(|e| format!("Failed to parse file: {}", e))?;

    let mut visitor = SymbolVisitor::new(file_path.to_path_buf());
    visitor.visit_file(&syntax_tree);

    Ok(visitor.symbols)
}

/// Symbol index for fast lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolIndex {
    symbols: Vec<Symbol>,
    by_name: HashMap<String, Vec<usize>>,
    by_type: HashMap<SymbolType, Vec<usize>>,
    by_file: HashMap<PathBuf, Vec<usize>>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            by_name: HashMap::new(),
            by_type: HashMap::new(),
            by_file: HashMap::new(),
        }
    }

    /// Add a symbol to the index
    pub fn add_symbol(&mut self, symbol: Symbol) {
        let idx = self.symbols.len();

        // Index by name
        self.by_name
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(idx);

        // Index by type
        self.by_type
            .entry(symbol.symbol_type.clone())
            .or_insert_with(Vec::new)
            .push(idx);

        // Index by file
        self.by_file
            .entry(symbol.path.clone())
            .or_insert_with(Vec::new)
            .push(idx);

        self.symbols.push(symbol);
    }

    /// Add multiple symbols to the index
    pub fn add_symbols(&mut self, symbols: Vec<Symbol>) {
        for symbol in symbols {
            self.add_symbol(symbol);
        }
    }

    /// Find symbols by name (exact match)
    pub fn find_by_name(&self, name: &str) -> Vec<&Symbol> {
        self.by_name
            .get(name)
            .map(|indices| indices.iter().map(|&i| &self.symbols[i]).collect())
            .unwrap_or_default()
    }

    /// Find symbols by name (partial match)
    pub fn find_by_name_partial(&self, pattern: &str) -> Vec<&Symbol> {
        let pattern_lower = pattern.to_lowercase();
        self.symbols
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&pattern_lower))
            .collect()
    }

    /// Find symbols by type
    pub fn find_by_type(&self, symbol_type: &SymbolType) -> Vec<&Symbol> {
        self.by_type
            .get(symbol_type)
            .map(|indices| indices.iter().map(|&i| &self.symbols[i]).collect())
            .unwrap_or_default()
    }

    /// Find symbols in a specific file
    pub fn find_in_file(&self, path: &Path) -> Vec<&Symbol> {
        self.by_file
            .get(path)
            .map(|indices| indices.iter().map(|&i| &self.symbols[i]).collect())
            .unwrap_or_default()
    }

    /// Get all symbols
    pub fn all_symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Get total symbol count
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.symbols.clear();
        self.by_name.clear();
        self.by_type.clear();
        self.by_file.clear();
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_symbols_function() {
        let code = r#"
            pub fn hello_world() {
                println!("Hello!");
            }
        "#;

        let symbols = extract_symbols(Path::new("test.rs"), code).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello_world");
        assert_eq!(symbols[0].symbol_type, SymbolType::Function);
        assert_eq!(symbols[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_extract_symbols_struct() {
        let code = r#"
            pub struct MyStruct {
                field: i32,
            }
        "#;

        let symbols = extract_symbols(Path::new("test.rs"), code).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyStruct");
        assert_eq!(symbols[0].symbol_type, SymbolType::Struct);
    }

    #[test]
    fn test_extract_symbols_enum() {
        let code = r#"
            pub enum Status {
                Active,
                Inactive,
            }
        "#;

        let symbols = extract_symbols(Path::new("test.rs"), code).unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Status");
        assert_eq!(symbols[0].symbol_type, SymbolType::Enum);
    }

    #[test]
    fn test_extract_symbols_trait() {
        let code = r#"
            pub trait MyTrait {
                fn do_something(&self);
            }
        "#;

        let symbols = extract_symbols(Path::new("test.rs"), code).unwrap();
        // Should find both the trait and the function
        assert!(symbols.len() >= 1);
        let trait_symbol = symbols.iter().find(|s| s.symbol_type == SymbolType::Trait);
        assert!(trait_symbol.is_some());
        assert_eq!(trait_symbol.unwrap().name, "MyTrait");
    }

    #[test]
    fn test_symbol_index_add_and_find() {
        let mut index = SymbolIndex::new();

        let symbol = Symbol::new(
            "test_fn".to_string(),
            SymbolType::Function,
            PathBuf::from("test.rs"),
            10,
            Visibility::Public,
        );

        index.add_symbol(symbol);

        assert_eq!(index.len(), 1);
        let found = index.find_by_name("test_fn");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "test_fn");
    }

    #[test]
    fn test_symbol_index_find_by_type() {
        let mut index = SymbolIndex::new();

        index.add_symbol(Symbol::new(
            "fn1".to_string(),
            SymbolType::Function,
            PathBuf::from("test.rs"),
            10,
            Visibility::Public,
        ));

        index.add_symbol(Symbol::new(
            "MyStruct".to_string(),
            SymbolType::Struct,
            PathBuf::from("test.rs"),
            20,
            Visibility::Public,
        ));

        let functions = index.find_by_type(&SymbolType::Function);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "fn1");

        let structs = index.find_by_type(&SymbolType::Struct);
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "MyStruct");
    }

    #[test]
    fn test_symbol_index_partial_match() {
        let mut index = SymbolIndex::new();

        index.add_symbol(Symbol::new(
            "hello_world".to_string(),
            SymbolType::Function,
            PathBuf::from("test.rs"),
            10,
            Visibility::Public,
        ));

        index.add_symbol(Symbol::new(
            "hello_rust".to_string(),
            SymbolType::Function,
            PathBuf::from("test.rs"),
            20,
            Visibility::Public,
        ));

        let results = index.find_by_name_partial("hello");
        assert_eq!(results.len(), 2);
    }
}
