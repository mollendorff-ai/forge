# Mouvify Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed test test-validate test-calculate test-all clean clean-test

# Detect if upx is available
HAS_UPX := $(shell command -v upx 2> /dev/null)

help:
	@echo "Mouvify Forge - Available Commands"
	@echo ""
	@echo "Build Targets:"
	@echo "  make build              - Standard release build"
	@echo "  make build-static       - Static release build (musl, 1.2MB)"
	@echo "  make build-compressed   - Static + UPX compressed (440KB)"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test               - Run both validation and calculation tests"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean              - Remove build artifacts"
	@echo "  make clean-test         - Restore test-data to original state"

build:
	@echo "🔨 Building release binary..."
	@cargo build --release
	@echo "✅ Binary: target/release/mouvify-forge"
	@ls -lh target/release/mouvify-forge

build-static:
	@echo "🔨 Building static release binary (musl)..."
	@cargo build --release --target x86_64-unknown-linux-musl
	@echo "✅ Binary: target/x86_64-unknown-linux-musl/release/mouvify-forge"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge

build-compressed: build-static
	@echo ""
ifdef HAS_UPX
	@echo "🗜️  Compressing binary with UPX..."
	@upx --best --lzma target/x86_64-unknown-linux-musl/release/mouvify-forge
	@echo ""
	@echo "✨ Compressed binary ready!"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge
else
	@echo "⚠️  UPX not found - install with: sudo apt install upx-ucl"
	@echo "📦 Static binary built (not compressed):"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge
endif

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

test: test-validate test-calculate
	@echo ""
	@echo "🎉 All tests passed!"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete!"

clean-test:
	@echo "🔄 Restoring test-data files to git state..."
	@git checkout test-data/*.yaml
	@echo "✅ Test data restored!"
