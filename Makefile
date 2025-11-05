# Questicle top-level Makefile
#
# Targets:
#   make build         - build Rust (qk, qk-lsp) and VS Code extension (compile TS)
#   make build-rust    - build Rust binaries (debug by default)
#   make build-ext     - install deps and compile VS Code extension TS
#   make package-ext   - package VS Code extension into a .vsix using npx @vscode/vsce
#   make install       - install Rust binaries (cargo install) and VS Code extension (.vsix)
#   make install-rust  - cargo install qk and qk-lsp into ~/.cargo/bin
#   make install-ext   - install extension into VS Code using `code --install-extension`
#   make clean         - clean Rust target and extension build artifacts

SHELL := /bin/sh

# Paths
EXT_DIR := editors/vscode/questicle-vscode

# Tools (override on command line if needed)
CARGO ?= cargo
NPM ?= npm
NPX ?= npx
CODE ?= code

# Determine extension metadata via Node (requires Node to be installed)
ext_publisher := $(shell node -p "require('./$(EXT_DIR)/package.json').publisher" 2>/dev/null)
ext_name := $(shell node -p "require('./$(EXT_DIR)/package.json').name" 2>/dev/null)
ext_version := $(shell node -p "require('./$(EXT_DIR)/package.json').version" 2>/dev/null)

# Expected packaged VSIX output name: <publisher>.<name>-<version>.vsix
VSIX := $(EXT_DIR)/$(ext_publisher).$(ext_name)-$(ext_version).vsix

.PHONY: help
help:
	@echo "Questicle Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  make build         - build Rust (qk, qk-lsp) and VS Code extension (compile TS)"
	@echo "  make build-rust    - build Rust binaries (debug by default)"
	@echo "  make build-ext     - install deps and compile VS Code extension TS"
	@echo "  make package-ext   - package VS Code extension into a .vsix"
	@echo "  make install       - install Rust binaries and VS Code extension"
	@echo "  make install-rust  - cargo install qk and qk-lsp into ~/.cargo/bin"
	@echo "  make install-ext   - install extension into VS Code using code CLI"
	@echo "  make clean         - clean Rust target and extension build artifacts"

.PHONY: build
build: build-rust build-ext

.PHONY: build-rust
build-rust:
	$(CARGO) build

.PHONY: build-ext
build-ext:
	cd $(EXT_DIR) && $(NPM) install && $(NPM) run compile

.PHONY: package-ext
package-ext: build-ext
	@echo "Packaging VS Code extension using @vscode/vsce..."
	cd $(EXT_DIR) && $(NPX) --yes @vscode/vsce package -o "$(abspath $(VSIX))"
	@echo "VSIX created at: $(VSIX)"

.PHONY: install
install: install-rust install-ext

.PHONY: install-rust
install-rust:
	@echo "Installing qk and qk-lsp to ~/.cargo/bin via cargo install..."
	$(CARGO) install --path . --force
	@echo "Ensure ~/.cargo/bin is on your PATH for the VS Code extension to find qk-lsp."

.PHONY: install-ext
install-ext: package-ext
	@echo "Installing VS Code extension via '$(CODE) --install-extension'..."
	$(CODE) --install-extension "$(VSIX)" --force
	@echo "If 'code' is not found, open VS Code and run: Command Palette -> 'Shell Command: Install \"code\" command in PATH'"

.PHONY: clean
clean:
	$(CARGO) clean
	@echo "Cleaning VS Code extension build output..."
	rm -rf $(EXT_DIR)/out
	rm -f $(EXT_DIR)/*.vsix
