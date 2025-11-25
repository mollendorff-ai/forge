//! Document management for Forge LSP
//!
//! Handles document caching, change tracking, and analysis.

use ropey::Rope;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Represents a cached Forge document
#[derive(Debug)]
pub struct ForgeDocument {
    /// Document content as a rope (efficient for edits)
    pub content: Rope,
    /// Document URI
    pub uri: Url,
    /// Document version
    pub version: i32,
    /// Cached analysis results
    pub analysis: Option<DocumentAnalysis>,
}

/// Analysis results for a document
#[derive(Debug, Clone)]
pub struct DocumentAnalysis {
    /// Variable definitions found in the document
    pub variables: Vec<VariableInfo>,
    /// Table definitions
    pub tables: Vec<TableInfo>,
    /// Formula references
    pub references: Vec<ReferenceInfo>,
    /// Validation errors
    pub errors: Vec<DocumentError>,
}

/// Information about a variable
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub line: u32,
    pub column: u32,
    pub var_type: VariableType,
    pub formula: Option<String>,
    pub value: Option<f64>,
}

/// Type of variable
#[derive(Debug, Clone, PartialEq)]
pub enum VariableType {
    Scalar,
    TableColumn,
    Aggregation,
}

/// Information about a table
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub line: u32,
    pub columns: Vec<String>,
    pub row_count: usize,
}

/// Information about a reference
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub target: String,
    pub line: u32,
    pub column: u32,
    pub ref_type: ReferenceType,
}

/// Type of reference
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    Variable,
    TableColumn,
    CrossFile,
    Function,
}

/// Document error
#[derive(Debug, Clone)]
pub struct DocumentError {
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub severity: DiagnosticSeverity,
}

impl ForgeDocument {
    /// Create a new document
    pub fn new(uri: Url, content: &str, version: i32) -> Self {
        Self {
            content: Rope::from_str(content),
            uri,
            version,
            analysis: None,
        }
    }

    /// Update document content
    pub fn update(&mut self, content: &str, version: i32) {
        self.content = Rope::from_str(content);
        self.version = version;
        self.analysis = None; // Invalidate cache
    }

    /// Get document text
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.content.len_lines()
    }

    /// Get line at index
    pub fn line(&self, index: usize) -> Option<String> {
        if index < self.content.len_lines() {
            Some(self.content.line(index).to_string())
        } else {
            None
        }
    }
}

/// Document store for all open documents
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<Url, ForgeDocument>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn open(&mut self, uri: Url, content: &str, version: i32) {
        let doc = ForgeDocument::new(uri.clone(), content, version);
        self.documents.insert(uri, doc);
    }

    pub fn update(&mut self, uri: &Url, content: &str, version: i32) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.update(content, version);
        }
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<&ForgeDocument> {
        self.documents.get(uri)
    }

    pub fn get_mut(&mut self, uri: &Url) -> Option<&mut ForgeDocument> {
        self.documents.get_mut(uri)
    }
}
