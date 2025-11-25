//! LSP Capabilities for Forge
//!
//! Defines what features the Forge LSP server supports.

use tower_lsp::lsp_types::*;

/// Get the server capabilities for Forge LSP
pub fn get_server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // Full document sync - we need the complete document for validation
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),

        // Completion for variables and functions
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                ".".to_string(),  // table.column
                "=".to_string(),  // formula start
                "@".to_string(),  // cross-file reference
                "(".to_string(),  // function call
            ]),
            resolve_provider: Some(true),
            ..Default::default()
        }),

        // Hover for calculated values and function docs
        hover_provider: Some(HoverProviderCapability::Simple(true)),

        // Go to definition for variable references
        definition_provider: Some(OneOf::Left(true)),

        // Find references
        references_provider: Some(OneOf::Left(true)),

        // Document symbols (outline)
        document_symbol_provider: Some(OneOf::Left(true)),

        // Real-time diagnostics
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
            DiagnosticOptions {
                identifier: Some("forge".to_string()),
                inter_file_dependencies: true,
                workspace_diagnostics: false,
                ..Default::default()
            },
        )),

        // Signature help for functions
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: Some(vec![",".to_string()]),
            ..Default::default()
        }),

        // Semantic tokens for syntax highlighting
        semantic_tokens_provider: Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: vec![
                        SemanticTokenType::VARIABLE,
                        SemanticTokenType::FUNCTION,
                        SemanticTokenType::NUMBER,
                        SemanticTokenType::STRING,
                        SemanticTokenType::KEYWORD,
                        SemanticTokenType::COMMENT,
                        SemanticTokenType::OPERATOR,
                    ],
                    token_modifiers: vec![
                        SemanticTokenModifier::DECLARATION,
                        SemanticTokenModifier::DEFINITION,
                        SemanticTokenModifier::READONLY,
                    ],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: Some(true),
                ..Default::default()
            }),
        ),

        ..Default::default()
    }
}
