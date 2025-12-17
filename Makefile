# Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed build-all install install-user install-system uninstall install-forge install-forge-demo install-all cross-forge-demo cross-forge publish-demo lint lint-fix format format-check test test-unit test-integration test-e2e test-validate test-calculate test-all test-coverage coverage coverage-report coverage-ci validate-docs validate-yaml validate-diagrams validate-all install-tools clean clean-test pre-build post-build pre-commit check

# ═══════════════════════════════════════════════════════════════════════════════
# OS AND ARCHITECTURE DETECTION
# ═══════════════════════════════════════════════════════════════════════════════

UNAME_S := $(shell uname -s 2>/dev/null || echo Windows)
UNAME_M := $(shell uname -m 2>/dev/null || echo x86_64)

# Normalize architecture names
ifeq ($(UNAME_M),arm64)
    ARCH := aarch64
else ifeq ($(UNAME_M),aarch64)
    ARCH := aarch64
else
    ARCH := x86_64
endif

# Set platform-specific variables
ifeq ($(UNAME_S),Linux)
    PLATFORM := linux
    BUILD_TARGET := $(ARCH)-unknown-linux-musl
    STATIC_BINARY := target/$(BUILD_TARGET)/release/forge-demo
    TARGET_FLAG := --target $(BUILD_TARGET)
    UPX_SUPPORTED := true
else ifeq ($(UNAME_S),Darwin)
    PLATFORM := macos
    BUILD_TARGET := $(ARCH)-apple-darwin
    STATIC_BINARY := target/release/forge-demo
    TARGET_FLAG :=
    # UPX not supported on macOS - breaks code signing
    UPX_SUPPORTED := false
else ifneq (,$(findstring MINGW,$(UNAME_S)))
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge-demo.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else ifneq (,$(findstring MSYS,$(UNAME_S)))
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge-demo.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else ifeq ($(OS),Windows_NT)
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge-demo.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else
    PLATFORM := unknown
    BUILD_TARGET :=
    STATIC_BINARY := target/release/forge-demo
    TARGET_FLAG :=
    UPX_SUPPORTED := false
endif

# Detect if tools are available
HAS_UPX := $(shell command -v upx 2> /dev/null)
HAS_CROSS := $(shell command -v cross 2> /dev/null)

# Cross-compilation targets (for build-all and new cross-* targets)
CROSS_TARGETS := x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu
CROSS_TARGETS_ALL := aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu

# Detect if cargo-zigbuild is available
HAS_ZIGBUILD := $(shell command -v cargo-zigbuild 2> /dev/null)

help:
	@echo "🔥 Forge - Available Commands"
	@echo ""
	@echo "Platform: $(PLATFORM) ($(ARCH))"
	@echo "Target:   $(BUILD_TARGET)"
	@echo ""
	@echo "Build Targets:"
	@echo "  make build              - Standard release build (with pre/post checks)"
	@echo "  make build-demo         - Build forge-demo only (36 functions)"
	@echo "  make build-enterprise   - Build forge + forge-server + forge-mcp (159 functions)"
	@echo "  make build-static       - Static release build for current platform"
	@echo "  make build-compressed   - Static + UPX compressed (Linux/Windows only)"
	@echo "  make build-all          - Cross-compile for all platforms (requires cross-rs)"
	@echo ""
	@echo "Install Targets (to ~/bin):"
	@echo "  make install-forge      - Build forge (enterprise) + install to ~/bin"
	@echo "  make install-forge-demo - Build forge-demo + install to ~/bin"
	@echo "  make install-all        - Build both binaries + install to ~/bin"
	@echo ""
	@echo "System Install Targets:"
	@echo "  make install            - Install to /usr/local/bin (system-wide, requires sudo)"
	@echo "  make install-user       - Install to ~/.local/bin (user-only, no sudo)"
	@echo "  make install-system     - Same as install (system-wide)"
	@echo "  make uninstall          - Uninstall from both locations"
	@echo ""
	@echo "Cross-Platform Builds (cargo-zigbuild):"
	@echo "  make cross-forge-demo   - Build forge-demo for all 5 platforms → dist/"
	@echo "  make cross-forge        - Build forge (enterprise) for all platforms → dist/"
	@echo ""
	@echo "GitHub Release:"
	@echo "  make publish-demo       - Build + publish to GitHub (version from Cargo.toml)"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint               - Run pedantic clippy checks"
	@echo "  make lint-fix           - Auto-fix clippy warnings"
	@echo "  make format             - Format code with rustfmt"
	@echo "  make format-check       - Check formatting without modifying"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test               - Run all cargo tests (unit + inline tests)"
	@echo "  make test-unit          - Run unit tests only (--lib)"
	@echo "  make test-integration   - Run integration tests only"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test-all           - Run ALL unit tests (2,703 tests)"
	@echo ""
	@echo "E2E Tests (separate repository - ADR-027):"
	@echo "  See: https://github.com/royalbit/forge-e2e"
	@echo ""
	@echo "Coverage Targets (ADR-004: 100% MANDATORY):"
	@echo "  make coverage           - Run coverage, FAIL if < 100%"
	@echo "  make coverage-report    - Generate HTML coverage report"
	@echo "  make coverage-ci        - CI mode: FAIL if < 100% + lcov output"
	@echo ""
	@echo "Documentation:"
	@echo "  make docs-cli           - Generate CLI reference from --help (auto)"
	@echo "  make docs-cli-check     - Verify CLI docs are up to date (CI)"
	@echo "  make validate-docs      - Validate markdown files (markdownlint-cli2)"
	@echo "  make validate-yaml      - Validate YAML files (yamllint)"
	@echo "  make validate-all       - Run ALL validators (docs + yaml)"
	@echo ""
	@echo "Presentation:"
	@echo "  (moved to https://github.com/royalbit/asimov)"
	@echo ""
	@echo "Workflows:"
	@echo "  make pre-commit         - Full pre-commit check (format + lint + test + validate-all)"
	@echo "  make check              - Quick check during development (faster than pre-commit)"
	@echo ""
	@echo "Utilities:"
	@echo "  make install-tools      - Show installation commands for required tools"
	@echo "  make clean              - Remove build artifacts"
	@echo "  make clean-test         - Restore test-data to original state"

pre-build:
	@echo "🔍 Running pre-build checks..."
	@echo ""
	@echo "1️⃣  Running lint (pedantic clippy)..."
	@$(MAKE) -s lint
	@echo ""
	@echo "2️⃣  Running unit tests..."
	@cargo test --lib --quiet
	@echo "✅ Unit tests passed!"
	@echo ""
	@echo "3️⃣  Checking CLI docs are up to date..."
	@$(MAKE) -s docs-cli-check
	@echo ""
	@echo "✅ Pre-build checks complete!"
	@echo ""

post-build:
	@echo ""
	@echo "🧪 Running post-build checks..."
	@echo ""
	@echo "1️⃣  Running E2E tests..."
	@cargo test --quiet
	@echo "✅ All tests passed!"
	@echo ""
	@echo "✅ Post-build checks complete!"

build: pre-build
	@echo "🔨 Building release binary..."
	@cargo build --release
	@echo "✅ Binary: target/release/forge-demo"
	@ls -lh target/release/forge-demo
	@$(MAKE) -s post-build

# Build demo binary only (36 functions, no servers)
build-demo:
	@echo "🔨 Building forge-demo (36 functions)..."
	@cargo build --release --bin forge-demo
	@echo "✅ Binary: target/release/forge-demo"
	@ls -lh target/release/forge-demo
	@echo ""
	@echo "📊 Function count:"
	@./target/release/forge-demo functions 2>/dev/null | wc -l | xargs -I{} echo "   {} functions available"

# Build enterprise binaries (173 functions + servers)
build-enterprise:
	@echo "🔨 Building enterprise binaries (173 functions)..."
	@cargo build --release
	@echo "✅ Binaries:"
	@ls -lh target/release/forge target/release/forge-server target/release/forge-mcp 2>/dev/null || true
	@echo ""
	@echo "📊 Function count:"
	@./target/release/forge functions 2>/dev/null | wc -l | xargs -I{} echo "   {} functions available"

build-static:
	@echo "🔨 Building static release binary..."
	@echo "   Platform: $(PLATFORM) ($(ARCH))"
	@echo "   Target:   $(BUILD_TARGET)"
ifeq ($(PLATFORM),linux)
	@cargo build --release $(TARGET_FLAG)
else ifeq ($(PLATFORM),macos)
	@cargo build --release
else ifeq ($(PLATFORM),windows)
	@cargo build --release
else
	@echo "❌ Unknown platform: $(UNAME_S)"
	@exit 1
endif
	@echo "✅ Binary: $(STATIC_BINARY)"
	@ls -lh $(STATIC_BINARY)

build-compressed: build-static
	@echo ""
ifeq ($(UPX_SUPPORTED),true)
ifdef HAS_UPX
	@echo "📦 BEFORE compression:"
	@ls -lh $(STATIC_BINARY) | tail -1
	@BEFORE=$$(stat -c%s $(STATIC_BINARY) 2>/dev/null || stat -f%z $(STATIC_BINARY)); \
	echo ""; \
	echo "🗜️  Compressing with UPX --best --lzma..."; \
	upx --best --lzma $(STATIC_BINARY); \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo "✨ AFTER compression:"; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	ls -lh $(STATIC_BINARY) | tail -1; \
	AFTER=$$(stat -c%s $(STATIC_BINARY) 2>/dev/null || stat -f%z $(STATIC_BINARY)); \
	SAVED=$$(($$BEFORE - $$AFTER)); \
	PERCENT=$$(awk "BEGIN {printf \"%.1f\", ($$SAVED / $$BEFORE) * 100}"); \
	echo ""; \
	echo "🎉 Saved: $$SAVED bytes ($$PERCENT% smaller!)"; \
	echo "📊 From $$(numfmt --to=iec-i --suffix=B $$BEFORE 2>/dev/null || echo $$BEFORE bytes) → $$(numfmt --to=iec-i --suffix=B $$AFTER 2>/dev/null || echo $$AFTER bytes)"
else
	@echo "⚠️  UPX not found - install with: sudo apt install upx-ucl (Linux) or choco install upx (Windows)"
	@echo "📦 Static binary built (not compressed):"
	@ls -lh $(STATIC_BINARY)
endif
else
	@echo "ℹ️  UPX compression not supported on $(PLATFORM) (breaks code signing)"
	@echo "📦 Static binary built:"
	@ls -lh $(STATIC_BINARY)
endif

# Cross-compile forge-demo for all platforms (requires cross-rs: cargo install cross)
build-all:
	@echo "🌍 Cross-compiling forge-demo for all platforms..."
	@echo ""
ifndef HAS_CROSS
	@echo "❌ cross-rs not found. Install with: cargo install cross"
	@echo "   Also requires Docker to be running."
	@exit 1
endif
	@mkdir -p dist
	@for target in $(CROSS_TARGETS); do \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		echo "🔨 Building forge-demo for $$target..."; \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		cross build --release --bin forge-demo --target $$target || exit 1; \
		if echo "$$target" | grep -q "windows"; then \
			cp target/$$target/release/forge-demo.exe dist/forge-demo-$$target.exe; \
			ls -lh dist/forge-demo-$$target.exe; \
		else \
			cp target/$$target/release/forge-demo dist/forge-demo-$$target; \
			ls -lh dist/forge-demo-$$target; \
		fi; \
		echo ""; \
	done
	@echo "✅ All builds complete! Binaries in dist/"
	@ls -lh dist/

install-system: clean build-compressed
	@echo "📦 Installing forge-demo to /usr/local/bin (system-wide)..."
ifeq ($(PLATFORM),windows)
	@echo "❌ Use install-user on Windows or copy manually"
	@exit 1
else
	@sudo install -m 755 $(STATIC_BINARY) /usr/local/bin/forge-demo
	@echo "✅ Installed to /usr/local/bin/forge-demo"
	@echo "🔍 Verify with: forge-demo --version"
endif

install-user: clean build-compressed
	@echo "📦 Installing forge-demo to ~/.local/bin (user-only)..."
	@mkdir -p ~/.local/bin
ifeq ($(PLATFORM),windows)
	@copy $(STATIC_BINARY) %USERPROFILE%\.local\bin\forge-demo.exe
else
	@install -m 755 $(STATIC_BINARY) ~/.local/bin/forge-demo
endif
	@echo "✅ Installed to ~/.local/bin/forge-demo"
	@echo "💡 Make sure ~/.local/bin is in your PATH"
	@echo "🔍 Verify with: forge-demo --version"

install: install-system

uninstall:
	@echo "🗑️  Uninstalling forge-demo..."
	@sudo rm -f /usr/local/bin/forge-demo 2>/dev/null || true
	@rm -f ~/.local/bin/forge-demo 2>/dev/null || true
	@echo "✅ Uninstalled from both /usr/local/bin and ~/.local/bin"

# ═══════════════════════════════════════════════════════════════════════════
# INSTALL TO ~/bin TARGETS
# ═══════════════════════════════════════════════════════════════════════════

install-forge:
	@echo "🔨 Building forge (enterprise)..."
	@cargo build --release --bin forge
	@echo ""
	@echo "📦 Installing forge to ~/bin..."
	@mkdir -p ~/bin
	@install -m 755 target/release/forge ~/bin/forge
	@echo "✅ Installed to ~/bin/forge"
	@echo "💡 Make sure ~/bin is in your PATH"
	@echo "🔍 Verify with: forge --version"
	@echo ""
	@echo "📊 Function count:"
	@~/bin/forge functions 2>/dev/null | wc -l | xargs -I{} echo "   {} functions available"

install-forge-demo:
	@echo "🔨 Building forge-demo (--features demo)..."
	@cargo build --release --bin forge-demo --features demo
	@echo ""
	@echo "📦 Installing forge-demo to ~/bin..."
	@mkdir -p ~/bin
	@install -m 755 target/release/forge-demo ~/bin/forge-demo
	@echo "✅ Installed to ~/bin/forge-demo"
	@echo "💡 Make sure ~/bin is in your PATH"
	@echo "🔍 Verify with: forge-demo --version"
	@echo ""
	@echo "📊 Function count:"
	@~/bin/forge-demo functions 2>/dev/null | wc -l | xargs -I{} echo "   {} functions available"

install-all: install-forge install-forge-demo
	@echo ""
	@echo "✅ All binaries installed to ~/bin!"
	@ls -lh ~/bin/forge ~/bin/forge-demo

# ═══════════════════════════════════════════════════════════════════════════
# CROSS-PLATFORM BUILDS (cargo-zigbuild)
# ═══════════════════════════════════════════════════════════════════════════

cross-forge-demo:
	@echo "🌍 Cross-compiling forge-demo for all platforms..."
	@echo ""
ifndef HAS_ZIGBUILD
	@echo "❌ cargo-zigbuild not found. Install with: cargo install cargo-zigbuild"
	@exit 1
endif
	@mkdir -p dist
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge-demo for macOS ARM64 (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge-demo --features demo --target aarch64-apple-darwin
	@cp target/aarch64-apple-darwin/release/forge-demo dist/forge-demo-aarch64-apple-darwin
	@ls -lh dist/forge-demo-aarch64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge-demo for macOS Intel (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge-demo --features demo --target x86_64-apple-darwin
	@cp target/x86_64-apple-darwin/release/forge-demo dist/forge-demo-x86_64-apple-darwin
	@ls -lh dist/forge-demo-x86_64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge-demo for Linux x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge-demo --features demo --target x86_64-unknown-linux-musl
	@cp target/x86_64-unknown-linux-musl/release/forge-demo dist/forge-demo-x86_64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-demo-x86_64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-demo-x86_64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge-demo for Linux ARM64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge-demo --features demo --target aarch64-unknown-linux-musl
	@cp target/aarch64-unknown-linux-musl/release/forge-demo dist/forge-demo-aarch64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-demo-aarch64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-demo-aarch64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🪟 Building forge-demo for Windows x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge-demo --features demo --target x86_64-pc-windows-gnu
	@cp target/x86_64-pc-windows-gnu/release/forge-demo.exe dist/forge-demo-x86_64-pc-windows-gnu.exe
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-demo-x86_64-pc-windows-gnu.exe; \
	fi
	@ls -lh dist/forge-demo-x86_64-pc-windows-gnu.exe
	@echo ""
	@echo "✅ All builds complete! Binaries in dist/"
	@ls -lh dist/forge-demo-*

cross-forge:
	@echo "🌍 Cross-compiling forge (enterprise) for all platforms..."
	@echo ""
ifndef HAS_ZIGBUILD
	@echo "❌ cargo-zigbuild not found. Install with: cargo install cargo-zigbuild"
	@exit 1
endif
	@mkdir -p dist
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge for macOS ARM64 (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge --target aarch64-apple-darwin
	@cp target/aarch64-apple-darwin/release/forge dist/forge-aarch64-apple-darwin
	@ls -lh dist/forge-aarch64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge for macOS Intel (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge --target x86_64-apple-darwin
	@cp target/x86_64-apple-darwin/release/forge dist/forge-x86_64-apple-darwin
	@ls -lh dist/forge-x86_64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge for Linux x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target x86_64-unknown-linux-musl
	@cp target/x86_64-unknown-linux-musl/release/forge dist/forge-x86_64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-x86_64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-x86_64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge for Linux ARM64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target aarch64-unknown-linux-musl
	@cp target/aarch64-unknown-linux-musl/release/forge dist/forge-aarch64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-aarch64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-aarch64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🪟 Building forge for Windows x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target x86_64-pc-windows-gnu
	@cp target/x86_64-pc-windows-gnu/release/forge.exe dist/forge-x86_64-pc-windows-gnu.exe
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-x86_64-pc-windows-gnu.exe; \
	fi
	@ls -lh dist/forge-x86_64-pc-windows-gnu.exe
	@echo ""
	@echo "✅ All builds complete! Binaries in dist/"
	@ls -lh dist/forge-*

# ═══════════════════════════════════════════════════════════════════════════
# GITHUB RELEASE PUBLISHING
# ═══════════════════════════════════════════════════════════════════════════

# Extract version from Cargo.toml
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

publish-demo:
	@echo "🚀 Publishing forge-demo v$(VERSION) to GitHub..."
	@echo ""
	@if ! command -v gh >/dev/null 2>&1; then \
		echo "❌ GitHub CLI (gh) not found. Install from: https://cli.github.com/"; \
		exit 1; \
	fi
	@echo "1️⃣  Building all platform binaries..."
	@$(MAKE) cross-forge-demo
	@echo ""
	@echo "2️⃣  Renaming binaries to release format..."
	@cp dist/forge-demo-aarch64-apple-darwin dist/forge-demo-$(VERSION)-darwin-arm64
	@cp dist/forge-demo-x86_64-apple-darwin dist/forge-demo-$(VERSION)-darwin-x86_64
	@cp dist/forge-demo-x86_64-unknown-linux-musl dist/forge-demo-$(VERSION)-linux-x86_64
	@cp dist/forge-demo-aarch64-unknown-linux-musl dist/forge-demo-$(VERSION)-linux-arm64
	@cp dist/forge-demo-x86_64-pc-windows-gnu.exe dist/forge-demo-$(VERSION)-windows-x86_64.exe
	@ls -lh dist/forge-demo-$(VERSION)-*
	@echo ""
	@echo "3️⃣  Creating GitHub release v$(VERSION)..."
	@gh release create "v$(VERSION)" \
		--repo royalbit/forge-demo \
		--title "forge-demo v$(VERSION)" \
		--generate-notes \
		dist/forge-demo-$(VERSION)-darwin-arm64 \
		dist/forge-demo-$(VERSION)-darwin-x86_64 \
		dist/forge-demo-$(VERSION)-linux-x86_64 \
		dist/forge-demo-$(VERSION)-linux-arm64 \
		dist/forge-demo-$(VERSION)-windows-x86_64.exe
	@echo ""
	@echo "✅ Release v$(VERSION) published!"
	@echo "🔗 View at: https://github.com/royalbit/forge-demo/releases/tag/v$(VERSION)"

lint:
	@echo "🔍 Running pedantic clippy checks..."
	@cargo clippy --all-targets --all-features -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions
	@echo "✅ Clippy checks passed!"

lint-fix:
	@echo "🔧 Running clippy with auto-fix..."
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions
	@echo "✅ Auto-fix complete!"

test-validate:
	@echo "🔍 Validating all test-data files..."
	@echo ""
	@for file in test-data/*.yaml; do \
		echo "📄 Validating: $$file"; \
		cargo run --release -- validate $$file || exit 1; \
		echo ""; \
	done
	@echo "✅ All test files validated successfully!"

test-calculate:
	@echo "🧮 Testing calculation on all test-data files (dry-run)..."
	@echo ""
	@for file in test-data/*.yaml; do \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		echo "📄 Calculating: $$file"; \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		cargo run --release -- calculate $$file --dry-run --verbose || exit 1; \
		echo ""; \
	done
	@echo "✅ All test calculations completed successfully!"

test:
	@echo "🧪 Running all cargo tests..."
	@cargo test

test-unit:
	@echo "🧪 Running unit tests..."
	@cargo test --lib

test-integration:
	@echo "🧪 Running integration tests..."
	@cargo test --test validation_tests

# E2E tests migrated to forge-e2e (ADR-027)
# See: https://github.com/royalbit/forge-e2e

test-all: test test-validate test-calculate
	@echo ""
	@echo "🎉 All tests passed!"

# Legacy test-coverage target (shows summary only)
test-coverage:
	@echo "📊 Test Coverage Summary (use 'make coverage' for actual coverage)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "See ADR-004: 100% test coverage is MANDATORY"
	@echo "Run 'make coverage' to verify coverage meets 100% requirement"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo test 2>&1 | grep -E "running [0-9]+ tests" | awk '{sum += $$2} END {print "Total tests: " sum}'

# ═══════════════════════════════════════════════════════════════════════════
# COVERAGE TARGETS (ADR-004: 100% REQUIRED)
# ═══════════════════════════════════════════════════════════════════════════

# Coverage: Run tests with coverage, FAIL if < 100%
# ADR-004: 100% coverage is MANDATORY - NO EXCEPTIONS
coverage:
	@echo "📊 Running test coverage (100% REQUIRED - ADR-004)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "❌ cargo-llvm-cov not found. Installing..."; \
		cargo install cargo-llvm-cov; \
	fi
	@cargo llvm-cov --fail-under-lines 100 --ignore-filename-regex '(tests/|test_)' || \
		(echo ""; echo "❌ COVERAGE BELOW 100% - BUILD FAILED (ADR-004)"; echo "Run 'make coverage-report' to see uncovered lines"; exit 1)
	@echo ""
	@echo "✅ 100% coverage verified!"

# Coverage report: Generate detailed HTML report and open in browser
coverage-report:
	@echo "📊 Generating coverage report..."
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "❌ cargo-llvm-cov not found. Installing..."; \
		cargo install cargo-llvm-cov; \
	fi
	@cargo llvm-cov --html --ignore-filename-regex '(tests/|test_)' --output-dir coverage-report
	@echo "✅ Coverage report generated: coverage-report/html/index.html"
	@if command -v xdg-open >/dev/null 2>&1; then \
		xdg-open coverage-report/html/index.html; \
	elif command -v open >/dev/null 2>&1; then \
		open coverage-report/html/index.html; \
	else \
		echo "Open coverage-report/html/index.html in your browser"; \
	fi

# Coverage CI: Strict 100% enforcement for CI/CD pipeline
# ADR-004: FAIL THE BUILD if < 100% - NO EXCEPTIONS
coverage-ci:
	@echo "📊 CI Coverage Check (100% REQUIRED - ADR-004)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo llvm-cov --fail-under-lines 100 --ignore-filename-regex '(tests/|test_)' --lcov --output-path lcov.info
	@echo ""
	@echo "✅ 100% coverage verified!"
	@echo "📄 lcov.info generated for coverage upload"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete!"

clean-test:
	@echo "🔄 Restoring test-data files to git state..."
	@git checkout test-data/*.yaml
	@echo "✅ Test data restored!"

# ═══════════════════════════════════════════════════════════════════════════
# CODE FORMATTING TARGETS
# ═══════════════════════════════════════════════════════════════════════════

format:
	@echo "🎨 Formatting code..."
	@cargo fmt
	@echo "✅ Code formatted"

format-check:
	@echo "🎨 Checking code formatting..."
	@cargo fmt -- --check
	@echo "✅ Code formatting is correct"

# ═══════════════════════════════════════════════════════════════════════════
# DOCUMENTATION VALIDATION TARGETS
# ═══════════════════════════════════════════════════════════════════════════

validate-docs:
	@echo "📝 Validating markdown files..."
	@if command -v markdownlint-cli2 >/dev/null 2>&1; then \
		markdownlint-cli2 '**/*.md' --config .markdownlint.json && \
		echo "✅ Markdown validation passed"; \
	else \
		echo "❌ markdownlint-cli2 not found. Run: npm install -g markdownlint-cli2"; \
		exit 1; \
	fi

validate-yaml:
	@echo "📄 Validating YAML files..."
	@if command -v yamllint >/dev/null 2>&1; then \
		yamllint warmup.yaml sprint.yaml roadmap.yaml 2>/dev/null && \
		echo "✅ YAML validation passed"; \
	else \
		echo "❌ yamllint not found. Run: pip install yamllint"; \
		exit 1; \
	fi

validate-diagrams:
	@echo "🎨 Diagram validation (Mermaid diagrams are validated by GitHub)"
	@echo "✅ Mermaid diagrams embedded in markdown - no validation needed"
	@if [ -d "diagrams" ] && find diagrams -name "*.puml" -o -name "*.plantuml" 2>/dev/null | grep -q .; then \
		echo "⚠️  Warning: Found old PlantUML files in diagrams/ - consider removing"; \
	fi

validate-all: validate-docs validate-yaml validate-diagrams
	@echo ""
	@echo "✅ All validation checks completed!"

# ═══════════════════════════════════════════════════════════════════════════
# DOCUMENTATION GENERATION
# ═══════════════════════════════════════════════════════════════════════════

# Generate CLI reference documentation from actual --help output
docs-cli:
	@echo "📚 Generating CLI documentation from --help..."
	@mkdir -p docs/cli
	@echo "# Forge CLI Reference" > docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo "> Auto-generated from \`forge-demo --help\`. Do not edit manually." >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo "## Main Help" >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo '```' >> docs/cli/README.md
	@./target/release/forge-demo --help >> docs/cli/README.md
	@echo '```' >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@for cmd in calculate validate audit export import watch compare variance sensitivity goal-seek break-even update functions upgrade; do \
		echo "## $$cmd" >> docs/cli/README.md; \
		echo "" >> docs/cli/README.md; \
		echo '```' >> docs/cli/README.md; \
		./target/release/forge-demo $$cmd --help >> docs/cli/README.md; \
		echo '```' >> docs/cli/README.md; \
		echo "" >> docs/cli/README.md; \
	done
	@echo "✅ Generated docs/cli/README.md"

# Verify CLI docs are up to date (for CI)
docs-cli-check:
	@echo "🔍 Checking CLI documentation is up to date..."
	@$(MAKE) -s docs-cli
	@if git diff --quiet docs/cli/README.md; then \
		echo "✅ CLI documentation is up to date"; \
	else \
		echo "❌ CLI documentation is out of date!"; \
		echo "Run 'make docs-cli' to regenerate"; \
		exit 1; \
	fi

# ═══════════════════════════════════════════════════════════════════════════
# UTILITY TARGETS
# ═══════════════════════════════════════════════════════════════════════════

install-tools:
	@echo "📦 Required tools for Forge development:"
	@echo ""
	@echo "1. Rust toolchain (required)"
	@echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
	@echo ""
	@echo "2. markdownlint-cli2 (documentation validation)"
	@echo "   npm install -g markdownlint-cli2"
	@echo ""
	@echo "3. yamllint (YAML validation)"
	@echo "   pip install yamllint"
	@echo ""
	@echo "4. Marp CLI (presentation generation)"
	@echo "   npm install -g @marp-team/marp-cli"
	@echo ""
	@echo "5. PlantUML (diagram validation - optional)"
	@echo "   Using public server: https://www.plantuml.com/plantuml"
	@echo ""
	@echo "Current status:"
	@command -v cargo >/dev/null 2>&1 && echo "  ✅ Rust/Cargo installed" || echo "  ❌ Rust/Cargo not found"
	@command -v markdownlint-cli2 >/dev/null 2>&1 && echo "  ✅ markdownlint-cli2 installed" || echo "  ❌ markdownlint-cli2 not found"
	@command -v yamllint >/dev/null 2>&1 && echo "  ✅ yamllint installed" || echo "  ❌ yamllint not found"
	@command -v marp >/dev/null 2>&1 && echo "  ✅ Marp CLI installed" || echo "  ❌ Marp CLI not found"
	@curl -s --head --max-time 5 https://www.plantuml.com/plantuml/png/ >/dev/null 2>&1 && echo "  ✅ PlantUML server accessible" || echo "  ⚠️  PlantUML server unreachable"
	@echo ""

# ═══════════════════════════════════════════════════════════════════════════
# WORKFLOW TARGETS
# ═══════════════════════════════════════════════════════════════════════════

# Full pre-commit check (what CI would run)
# ADR-004: 100% coverage is MANDATORY - NO EXCEPTIONS
pre-commit: format-check lint test coverage docs-cli-check validate-all
	@echo ""
	@echo "✅ Pre-commit checks passed! Safe to commit."

# Quick check during development (faster than pre-commit)
check: format-check lint test-unit validate-docs
	@echo ""
	@echo "✅ Quick checks passed!"

# ═══════════════════════════════════════════════════════════════════════════
# PRESENTATION
# ═══════════════════════════════════════════════════════════════════════════
# Presentation deck moved to: https://github.com/royalbit/asimov
# See: docs/PRESENTATION.md for redirect info
