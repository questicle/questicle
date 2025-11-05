# Questicle VS Code Extension

This extension provides advanced language support for Questicle:
- Language Server: completion, hover (typed), signature help, diagnostics (parse + type), document symbols, formatting
- Syntax highlighting with type keywords and arrow operator
- Command: Questicle: Run Current File (Ctrl+Shift+R)

## Install locally

1. Build the server binary:

```
cargo build
```

2. Install dependencies and compile the extension:

```
cd editors/vscode/questicle-vscode
npm install
npm run compile
```

3. Launch VS Code in this folder and press F5 to run the extension in a new window.

Or package and install:

```
# if you have vsce installed
vsce package
code --install-extension questicle-vscode-0.0.1.vsix
```

The extension looks for the `qk-lsp` binary in `target/debug/qk-lsp` relative to the workspace root. You can override using the setting `questicle.serverPath`.

Format Document works out-of-the-box via the LSP. Use the Run command to execute the current .qk file in a terminal.
