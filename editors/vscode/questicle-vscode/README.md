# Questicle VS Code Extension

This extension provides basic language support for Questicle:
- Language Server (completion, hover, diagnostics, document symbols)
- Syntax highlighting

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

The extension looks for the `qk-lsp` binary in `target/debug/qk-lsp` relative to the workspace root. If you want to override the path, we can add a setting later.
