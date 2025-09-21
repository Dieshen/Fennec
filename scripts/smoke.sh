#!/bin/bash

# Fennec Smoke Test Script
# Performs basic validation of the Fennec CLI

set -euo pipefail

echo "Starting Fennec smoke test..."

# Check that binary builds
echo "Building Fennec..."
cargo build --bin fennec

# Check basic CLI functionality
echo "Testing CLI help..."
cargo run --bin fennec -- --help > /dev/null

echo "Testing configuration loading..."
# Create a temporary config
TEMP_CONFIG=$(mktemp)
cat > "$TEMP_CONFIG" <<EOF
[provider]
default_model = "gpt-4"
timeout_seconds = 30

[security]
default_sandbox_level = "workspace-write"
audit_log_enabled = true

[memory]
max_transcript_size = 10000
enable_agents_md = true

[tui]
theme = "default"

[tui.key_bindings]
quit = "Ctrl+C"
help = "F1"
clear = "Ctrl+L"
EOF

# Test config validation (this will fail gracefully without API key)
cargo run --bin fennec -- --config "$TEMP_CONFIG" --help > /dev/null

# Clean up
rm "$TEMP_CONFIG"

echo "Verifying workspace structure..."
[ -d "crates/fennec-cli" ] || { echo "Missing CLI crate"; exit 1; }
[ -d "crates/fennec-core" ] || { echo "Missing core crate"; exit 1; }
[ -d "crates/fennec-tui" ] || { echo "Missing TUI crate"; exit 1; }
[ -d "crates/fennec-orchestration" ] || { echo "Missing orchestration crate"; exit 1; }
[ -d "crates/fennec-memory" ] || { echo "Missing memory crate"; exit 1; }
[ -d "crates/fennec-provider" ] || { echo "Missing provider crate"; exit 1; }
[ -d "crates/fennec-security" ] || { echo "Missing security crate"; exit 1; }
[ -d "crates/fennec-commands" ] || { echo "Missing commands crate"; exit 1; }

echo "Checking AGENTS.md exists..."
[ -f "AGENTS.md" ] || { echo "Missing AGENTS.md file"; exit 1; }

echo "Verifying docs directory..."
[ -d "docs" ] || { echo "Missing docs directory"; exit 1; }
[ -f "docs/MVP.md" ] || { echo "Missing MVP roadmap"; exit 1; }

echo "Running basic tests..."
cargo test --lib

echo "✅ Smoke test completed successfully!"
echo "Fennec workspace is properly scaffolded and ready for development."