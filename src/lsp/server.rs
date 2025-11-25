//! Forge LSP Server implementation
//!
//! Powers all editor extensions from a single codebase.

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::core::ArrayCalculator;
use crate::error::ForgeError;
use crate::parser;

/// Forge Language Server
pub struct ForgeLsp {
    /// LSP client for sending notifications
    client: Client,
    /// Document contents cache
    documents: DashMap<Url, Rope>,
}

impl ForgeLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Validate a document and return diagnostics
    async fn validate_document(&self, uri: &Url) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Get document content
        let content = match self.documents.get(uri) {
            Some(doc) => doc.to_string(),
            None => return diagnostics,
        };

        // Try to parse and validate
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return diagnostics,
        };

        // Parse the model
        match parser::parse_model(&path) {
            Ok(model) => {
                // Try to calculate - errors will be reported as diagnostics
                let calculator = ArrayCalculator::new(model);
                if let Err(e) = calculator.calculate_all() {
                    let diagnostic = error_to_diagnostic(&e, &content);
                    diagnostics.push(diagnostic);
                }
            }
            Err(e) => {
                let diagnostic = error_to_diagnostic(&e, &content);
                diagnostics.push(diagnostic);
            }
        }

        diagnostics
    }

    /// Get completions for variables and functions
    fn get_completions(&self, uri: &Url, _position: Position) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        // Add Excel function completions (50+ functions)
        let functions = get_forge_functions();
        for (name, detail, doc) in functions {
            completions.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail.to_string()),
                documentation: Some(Documentation::String(doc.to_string())),
                insert_text: Some(format!("{}($0)", name)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }

        // Add variable completions from current document
        if let Some(doc) = self.documents.get(uri) {
            let content = doc.to_string();
            // Simple variable extraction - look for yaml keys
            for line in content.lines() {
                if let Some(key) = extract_yaml_key(line) {
                    if !key.starts_with('_') && !key.is_empty() {
                        completions.push(CompletionItem {
                            label: key.to_string(),
                            kind: Some(CompletionItemKind::VARIABLE),
                            detail: Some("Variable".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        completions
    }

    /// Get hover information for a position
    fn get_hover(&self, uri: &Url, position: Position) -> Option<Hover> {
        let doc = self.documents.get(uri)?;
        let content = doc.to_string();
        let lines: Vec<&str> = content.lines().collect();

        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        // Extract word at position
        let word = extract_word_at_position(line, char_pos)?;

        // Check if it's a function
        let functions = get_forge_functions();
        for (name, detail, doc) in functions {
            if name.eq_ignore_ascii_case(&word) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("**{}**\n\n{}\n\n{}", name, detail, doc),
                    }),
                    range: None,
                });
            }
        }

        // Try to get calculated value for variable
        if let Ok(path) = uri.to_file_path() {
            if let Ok(model) = parser::parse_model(&path) {
                // Check scalars
                for (name, var) in &model.scalars {
                    if name.ends_with(&word) || name == &word {
                        let mut info = format!("**{}**\n\nType: Scalar", name);
                        if let Some(value) = var.value {
                            info.push_str(&format!("\n\nValue: `{}`", value));
                        }
                        if let Some(ref formula) = var.formula {
                            info.push_str(&format!("\n\nFormula: `{}`", formula));
                        }
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: info,
                            }),
                            range: None,
                        });
                    }
                }

                // Check table columns
                for (table_name, table) in &model.tables {
                    for (col_name, col) in &table.columns {
                        if col_name == &word {
                            let row_count = col.values.len();
                            let mut info = format!(
                                "**{}**\n\nTable: `{}`\nRows: {}",
                                col_name, table_name, row_count
                            );
                            if let Some(formula) = table.row_formulas.get(col_name) {
                                info.push_str(&format!("\n\nFormula: `{}`", formula));
                            }
                            return Some(Hover {
                                contents: HoverContents::Markup(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: info,
                                }),
                                range: None,
                            });
                        }
                    }
                }
            }
        }

        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ForgeLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        "=".to_string(),
                        "@".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("forge".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Forge Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Forge LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), Rope::from_str(&text));

        // Validate and publish diagnostics
        let diagnostics = self.validate_document(&uri).await;
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(uri.clone(), Rope::from_str(&change.text));

            // Validate and publish diagnostics
            let diagnostics = self.validate_document(&uri).await;
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let completions = self.get_completions(&uri, position);
        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        Ok(self.get_hover(&uri, position))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc = match self.documents.get(&uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let content = doc.to_string();
        let lines: Vec<&str> = content.lines().collect();

        if position.line as usize >= lines.len() {
            return Ok(None);
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        // Extract word at position
        let word = match extract_word_at_position(line, char_pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Find definition in document
        for (line_num, line_content) in lines.iter().enumerate() {
            if let Some(key) = extract_yaml_key(line_content) {
                if key == word {
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: 0,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: line_content.len() as u32,
                            },
                        },
                    })));
                }
            }
        }

        Ok(None)
    }
}

/// Run the LSP server on stdin/stdout
pub async fn run_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ForgeLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Convert a ForgeError to an LSP Diagnostic
fn error_to_diagnostic(error: &ForgeError, _content: &str) -> Diagnostic {
    let message = error.to_string();

    // Try to extract line number from error message
    let line = 0u32; // Default to first line

    Diagnostic {
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: 100,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("forge".to_string()),
        message,
        ..Default::default()
    }
}

/// Extract YAML key from a line
fn extract_yaml_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with('-') {
        return None;
    }
    if let Some(colon_pos) = trimmed.find(':') {
        let key = trimmed[..colon_pos].trim();
        if !key.is_empty() && !key.contains(' ') {
            return Some(key.to_string());
        }
    }
    None
}

/// Extract word at a given position in a line
fn extract_word_at_position(line: &str, char_pos: usize) -> Option<String> {
    if char_pos >= line.len() {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();
    let mut start = char_pos;
    let mut end = char_pos;

    // Find word boundaries
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

/// Get all Forge functions with documentation
fn get_forge_functions() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Aggregation
        ("SUM", "SUM(number1, [number2], ...)", "Adds all numbers in a range"),
        ("AVERAGE", "AVERAGE(number1, [number2], ...)", "Returns the average of numbers"),
        ("COUNT", "COUNT(value1, [value2], ...)", "Counts the number of values"),
        ("MAX", "MAX(number1, [number2], ...)", "Returns the largest number"),
        ("MIN", "MIN(number1, [number2], ...)", "Returns the smallest number"),
        ("PRODUCT", "PRODUCT(number1, [number2], ...)", "Multiplies all numbers"),

        // Conditional
        ("SUMIF", "SUMIF(range, criteria, [sum_range])", "Sums values that meet criteria"),
        ("COUNTIF", "COUNTIF(range, criteria)", "Counts values that meet criteria"),
        ("AVERAGEIF", "AVERAGEIF(range, criteria, [average_range])", "Averages values that meet criteria"),
        ("SUMIFS", "SUMIFS(sum_range, criteria_range1, criteria1, ...)", "Sums values with multiple criteria"),
        ("COUNTIFS", "COUNTIFS(criteria_range1, criteria1, ...)", "Counts values with multiple criteria"),
        ("AVERAGEIFS", "AVERAGEIFS(average_range, criteria_range1, criteria1, ...)", "Averages with multiple criteria"),
        ("MAXIFS", "MAXIFS(max_range, criteria_range1, criteria1, ...)", "Max with criteria"),
        ("MINIFS", "MINIFS(min_range, criteria_range1, criteria1, ...)", "Min with criteria"),

        // Logical
        ("IF", "IF(condition, value_if_true, value_if_false)", "Conditional logic"),
        ("AND", "AND(logical1, [logical2], ...)", "Returns TRUE if all arguments are TRUE"),
        ("OR", "OR(logical1, [logical2], ...)", "Returns TRUE if any argument is TRUE"),
        ("NOT", "NOT(logical)", "Reverses logical value"),
        ("IFERROR", "IFERROR(value, value_if_error)", "Returns alternate value on error"),

        // Math
        ("ROUND", "ROUND(number, num_digits)", "Rounds to specified digits"),
        ("ROUNDUP", "ROUNDUP(number, num_digits)", "Rounds up"),
        ("ROUNDDOWN", "ROUNDDOWN(number, num_digits)", "Rounds down"),
        ("CEILING", "CEILING(number, significance)", "Rounds up to significance"),
        ("FLOOR", "FLOOR(number, significance)", "Rounds down to significance"),
        ("SQRT", "SQRT(number)", "Square root"),
        ("POWER", "POWER(number, power)", "Raises to power"),
        ("MOD", "MOD(number, divisor)", "Returns remainder"),
        ("ABS", "ABS(number)", "Absolute value"),

        // Text
        ("CONCAT", "CONCAT(text1, [text2], ...)", "Joins text strings"),
        ("UPPER", "UPPER(text)", "Converts to uppercase"),
        ("LOWER", "LOWER(text)", "Converts to lowercase"),
        ("TRIM", "TRIM(text)", "Removes extra spaces"),
        ("LEN", "LEN(text)", "Returns text length"),
        ("MID", "MID(text, start_num, num_chars)", "Extracts characters from middle"),

        // Date
        ("TODAY", "TODAY()", "Returns current date"),
        ("DATE", "DATE(year, month, day)", "Creates a date"),
        ("YEAR", "YEAR(date)", "Extracts year"),
        ("MONTH", "MONTH(date)", "Extracts month"),
        ("DAY", "DAY(date)", "Extracts day"),

        // Lookup
        ("MATCH", "MATCH(lookup_value, lookup_array, [match_type])", "Returns position of value"),
        ("INDEX", "INDEX(array, row_num, [column_num])", "Returns value at position"),
        ("XLOOKUP", "XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found])", "Modern lookup function"),
        ("VLOOKUP", "VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])", "Vertical lookup"),
    ]
}
