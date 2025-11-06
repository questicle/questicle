import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';
import { Executable, LanguageClient, LanguageClientOptions, ServerOptions, State as ClientState } from 'vscode-languageclient/node';

export async function activate(context: vscode.ExtensionContext) {
    const out = vscode.window.createOutputChannel('Questicle');
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return;
    }

    // Look for the qk-lsp binary relative to the workspace root
    const root = workspaceFolders[0].uri.fsPath;
    const config = vscode.workspace.getConfiguration('questicle');
    const custom = config.get<string>('serverPath');
    const exeName = process.platform === 'win32' ? 'qk-lsp.exe' : 'qk-lsp';
    const debugPath = path.join(root, 'target', 'debug', exeName);
    const releasePath = path.join(root, 'target', 'release', exeName);
    const candidates: string[] = [];
    if (custom && custom.length > 0) {
        candidates.push(custom);
    }
    candidates.push(debugPath, releasePath, exeName);
    const command = pickFirstExecutable(candidates);
    out.appendLine(`[Questicle] Using LSP server: ${command}`);

    const run: Executable = { command, options: { cwd: root } };
    const serverOptions: ServerOptions = { run, debug: run };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ language: 'questicle', scheme: 'file' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.qk')
        }
    };

    const client = new LanguageClient('questicle', 'Questicle Language Server', serverOptions, clientOptions);
    // Dispose the client when the extension deactivates
    context.subscriptions.push(client);
    // Start the language client (don't push the Promise)
    client.onDidChangeState((e) => {
        out.appendLine(`[Questicle] Client state changed: ${ClientState[e.newState]}`);
    });
    client.start();

    // Register Run Current File command
    const runCmd = vscode.commands.registerCommand('questicle.runFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) { return; }
        const doc = editor.document;
        if (doc.languageId !== 'questicle') { return; }
        await doc.save();
        const root = workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
        const qkExeName = process.platform === 'win32' ? 'qk.exe' : 'qk';
        const qkDebug = path.join(root, 'target', 'debug', qkExeName);
        const qkRelease = path.join(root, 'target', 'release', qkExeName);
        const cmd = pickFirstExecutable([qkDebug, qkRelease, qkExeName]);
        out.appendLine(`[Questicle] Running file with: ${cmd}`);
        const terminal = vscode.window.createTerminal({ name: 'Questicle', cwd: root });
        terminal.show(true);
        terminal.sendText(`${cmd} ${doc.uri.fsPath}`);
    });
    context.subscriptions.push(runCmd);

    // Register formatting provider using CLI (qk fmt --stdin)
    context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider({ language: 'questicle' }, {
        provideDocumentFormattingEdits: async (document, options, token) => {
            const root = workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
            const qkExeName = process.platform === 'win32' ? 'qk.exe' : 'qk';
            const qkDebug = path.join(root, 'target', 'debug', qkExeName);
            const qkRelease = path.join(root, 'target', 'release', qkExeName);
            const cmd = pickFirstExecutable([qkDebug, qkRelease, qkExeName]);
            const input = document.getText();
            const result = await runCli(cmd, ['fmt', '--stdin'], input, root);
            if (result === undefined) { return []; }
            return [vscode.TextEdit.replace(fullRange(document), result)];
        }
    }));

    // Expose a command to format explicitly
    const fmtCmd = vscode.commands.registerCommand('questicle.formatDocument', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'questicle') { return; }
        await vscode.commands.executeCommand('editor.action.formatDocument');
    });
    context.subscriptions.push(fmtCmd);
}

export function deactivate(): Thenable<void> | undefined {
    return undefined;
}

function fullRange(doc: vscode.TextDocument): vscode.Range {
    const lastLine = doc.lineCount - 1;
    return new vscode.Range(0, 0, lastLine, doc.lineAt(lastLine).text.length);
}

function pickFirstExecutable(candidates: string[]): string {
    for (const c of candidates) {
        try {
            // If absolute or relative path exists, prefer it
            if (c.includes(path.sep)) {
                if (fs.existsSync(c)) { return c; }
            } else {
                // bare command; hope it's on PATH
                return c;
            }
        } catch {
            // ignore and continue
        }
    }
    // Fallback to the last candidate
    return candidates[candidates.length - 1];
}

async function runCli(cmd: string, args: string[], stdin: string, cwd: string): Promise<string | undefined> {
    return new Promise((resolve) => {
        const cp = require('child_process').spawn(cmd, args, { cwd });
        let out = '';
        let err = '';
        cp.stdout.on('data', (d: Buffer) => out += d.toString());
        cp.stderr.on('data', (d: Buffer) => err += d.toString());
        cp.on('error', (e: Error) => {
            const ch = vscode.window.createOutputChannel('Questicle');
            ch.appendLine(`[Questicle] Failed to run ${cmd}: ${String(e)}`);
            if (err.length > 0) { ch.appendLine(err); }
            resolve(undefined);
        });
        cp.on('close', (_code: number) => resolve(out));
        cp.stdin.write(stdin);
        cp.stdin.end();
    });
}
