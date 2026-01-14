#!/bin/bash
# Test script to identify compilation issues

echo "=== Testing Cargo Build ==="
echo "Checking workspace structure..."

# Check if all required files exist
echo ""
echo "Checking required files..."
for file in Cargo.toml crates/proto/Cargo.toml crates/common/Cargo.toml crates/worker/Cargo.toml crates/coordinator/Cargo.toml; do
    if [ -f "$file" ]; then
        echo "✓ $file exists"
    else
        echo "✗ $file missing!"
    fi
done

echo ""
echo "Checking source files..."
for file in crates/worker/src/main.rs crates/worker/src/lib.rs crates/coordinator/src/main.rs crates/coordinator/src/lib.rs; do
    if [ -f "$file" ]; then
        echo "✓ $file exists"
    else
        echo "✗ $file missing!"
    fi
done

echo ""
echo "=== Checking for common issues ==="

# Check if Cargo.toml has correct format
echo ""
echo "Checking Cargo.toml workspace definition..."
if grep -q "\[workspace\]" Cargo.toml; then
    echo "✓ Workspace definition found"
else
    echo "✗ Workspace definition missing!"
fi

if grep -q "members = \[" Cargo.toml; then
    echo "✓ Workspace members defined"
else
    echo "✗ Workspace members not defined!"
fi

echo ""
echo "=== Checking binaries section ==="
for cargo_toml in crates/*/Cargo.toml; do
    echo ""
    echo "Checking $cargo_toml..."
    if grep -q "\[\[bin\]\]" "$cargo_toml"; then
        echo "  Has binary definition:"
        grep -A2 "\[\[bin\]\]" "$cargo_toml" | head -3
    else
        echo "  No binary definition (library only)"
    fi
done
