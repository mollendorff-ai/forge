/**
 * Forge YAML Extension for VSCode
 *
 * Provides language support for Forge YAML formula files:
 * - Syntax highlighting
 * - Real-time validation
 * - Autocomplete for variables and 50+ functions
 * - Hover to see calculated values
 * - Go to definition
 */

import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('Forge YAML extension is now active');

    // Get LSP server path from configuration
    const config = vscode.workspace.getConfiguration('forge');
    const lspPath = config.get<string>('lspPath', 'forge-lsp');

    // Server options - run the forge-lsp binary
    const serverOptions: ServerOptions = {
        run: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
        debug: {
            command: lspPath,
            transport: TransportKind.stdio,
        },
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        // Register for YAML files
        documentSelector: [
            { scheme: 'file', language: 'yaml' },
            { scheme: 'file', language: 'forge-yaml' },
            { scheme: 'file', pattern: '**/*.forge.yaml' },
            { scheme: 'file', pattern: '**/*.forge.yml' },
        ],
        synchronize: {
            // Synchronize configuration to the server
            configurationSection: 'forge',
            // Watch for .yaml file changes in the workspace
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.yaml'),
        },
    };

    // Create the language client
    client = new LanguageClient(
        'forge-lsp',
        'Forge Language Server',
        serverOptions,
        clientOptions
    );

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('forge.validate', validateCommand),
        vscode.commands.registerCommand('forge.calculate', calculateCommand),
        vscode.commands.registerCommand('forge.exportExcel', exportExcelCommand),
        vscode.commands.registerCommand('forge.showAudit', showAuditCommand)
    );

    // Start the client
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}

/**
 * Validate the current Forge YAML file
 */
async function validateCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    if (!isForgeYaml(document)) {
        vscode.window.showWarningMessage('Current file is not a Forge YAML file');
        return;
    }

    // Run forge validate
    const terminal = vscode.window.createTerminal('Forge');
    terminal.show();
    terminal.sendText(`forge validate "${document.fileName}"`);
}

/**
 * Calculate all formulas in the current file
 */
async function calculateCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    if (!isForgeYaml(document)) {
        vscode.window.showWarningMessage('Current file is not a Forge YAML file');
        return;
    }

    // Run forge calculate with dry-run
    const terminal = vscode.window.createTerminal('Forge');
    terminal.show();
    terminal.sendText(`forge calculate "${document.fileName}" --dry-run`);
}

/**
 * Export current file to Excel
 */
async function exportExcelCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    if (!isForgeYaml(document)) {
        vscode.window.showWarningMessage('Current file is not a Forge YAML file');
        return;
    }

    // Prompt for output file
    const outputUri = await vscode.window.showSaveDialog({
        defaultUri: vscode.Uri.file(document.fileName.replace('.yaml', '.xlsx')),
        filters: {
            'Excel Files': ['xlsx'],
        },
    });

    if (outputUri) {
        const terminal = vscode.window.createTerminal('Forge');
        terminal.show();
        terminal.sendText(`forge export "${document.fileName}" "${outputUri.fsPath}"`);
    }
}

/**
 * Show audit trail for selected variable
 */
async function showAuditCommand() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    if (!isForgeYaml(document)) {
        vscode.window.showWarningMessage('Current file is not a Forge YAML file');
        return;
    }

    // Get word under cursor
    const position = editor.selection.active;
    const wordRange = document.getWordRangeAtPosition(position);
    const word = wordRange ? document.getText(wordRange) : '';

    // Prompt for variable name
    const variable = await vscode.window.showInputBox({
        prompt: 'Enter variable name to audit',
        value: word,
    });

    if (variable) {
        const terminal = vscode.window.createTerminal('Forge');
        terminal.show();
        terminal.sendText(`forge audit "${document.fileName}" "${variable}"`);
    }
}

/**
 * Check if a document is a Forge YAML file
 */
function isForgeYaml(document: vscode.TextDocument): boolean {
    if (document.languageId === 'forge-yaml') {
        return true;
    }
    if (document.languageId === 'yaml') {
        // Check for .forge.yaml extension
        if (document.fileName.endsWith('.forge.yaml') || document.fileName.endsWith('.forge.yml')) {
            return true;
        }
        // Check for _forge_version in content
        const text = document.getText();
        if (text.includes('_forge_version') || text.includes('tables:') || text.includes('scalars:')) {
            return true;
        }
    }
    return false;
}
