import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';
import { Executable, LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

export async function activate(context: vscode.ExtensionContext) {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return;
    }

    // Look for the qk-lsp binary relative to the workspace root
    const root = workspaceFolders[0].uri.fsPath;
    const config = vscode.workspace.getConfiguration('questicle');
    const custom = config.get<string>('serverPath');
    const debugPath = process.platform === 'win32' ? path.join(root, 'target', 'debug', 'qk-lsp.exe') : path.join(root, 'target', 'debug', 'qk-lsp');
    const command = (custom && custom.length > 0) ? custom : (fs.existsSync(debugPath) ? debugPath : (process.platform === 'win32' ? 'qk-lsp.exe' : 'qk-lsp'));

    const run: Executable = { command, options: { cwd: root } };
    const serverOptions: ServerOptions = { run, debug: run };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ language: 'questicle', scheme: 'file' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.qk')
        }
    };

    const client = new LanguageClient('questicle', 'Questicle Language Server', serverOptions, clientOptions);
    context.subscriptions.push(client.start());
}

export function deactivate(): Thenable<void> | undefined {
    return undefined;
}
