import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
    const serverOptions = resolveServerOptions();
    if (!serverOptions) {
        vscode.window.showWarningMessage(
            'Marduk LSP: open the marduk project folder to enable the language server.'
        );
        return;
    }

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'mkml' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.mkml'),
        },
    };

    client = new LanguageClient('marduk-lsp', 'Marduk LSP', serverOptions, clientOptions);
    client.start();
    context.subscriptions.push(client);
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}

function resolveServerOptions(): ServerOptions | undefined {
    const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (!workspacePath) return undefined;

    const exe = process.platform === 'win32' ? 'marduk-lsp.exe' : 'marduk-lsp';

    // Prefer a pre-built binary (fast startup).
    for (const sub of ['release', 'debug']) {
        const bin = path.join(workspacePath, 'target', sub, exe);
        if (fs.existsSync(bin)) {
            return { command: bin };
        }
    }

    // Fall back to `cargo run` — slow on first launch while compiling.
    return {
        command: 'cargo',
        args: [
            'run',
            '--manifest-path', path.join(workspacePath, 'Cargo.toml'),
            '--package', 'marduk-lsp',
            '--quiet',
        ],
    };
}
