#!/bin/bash
# Test script for nvim-linthis

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

echo "=== nvim-linthis Test ==="
echo "Plugin dir: $SCRIPT_DIR"
echo "Test dir: $TEST_DIR"

# Create minimal Neovim config
mkdir -p "$TEST_DIR/nvim"
cat > "$TEST_DIR/nvim/init.lua" << 'EOF'
-- Add plugin to runtime path
vim.opt.rtp:prepend(vim.env.PLUGIN_DIR)

-- Setup linthis
require("linthis").setup({
  notifications = true,
  log_level = "debug",
})

-- Print loaded message
print("linthis.nvim loaded successfully!")

-- Run tests after startup
vim.defer_fn(function()
  print("\n=== Running Tests ===")

  -- Test 1: Check if module loads
  local ok, linthis = pcall(require, "linthis")
  if ok then
    print("[PASS] Module loads correctly")
  else
    print("[FAIL] Module failed to load: " .. tostring(linthis))
  end

  -- Test 2: Check config
  local config = require("linthis.config").get()
  if config and config.cmd then
    print("[PASS] Config initialized: cmd = " .. table.concat(config.cmd, " "))
  else
    print("[FAIL] Config not initialized")
  end

  -- Test 3: Check commands exist
  local commands = { "LinthisFormat", "LinthisLint", "LinthisRestart", "LinthisInfo" }
  for _, cmd in ipairs(commands) do
    if vim.fn.exists(":" .. cmd) == 2 then
      print("[PASS] Command :" .. cmd .. " exists")
    else
      print("[FAIL] Command :" .. cmd .. " not found")
    end
  end

  -- Test 4: Check executable
  if vim.fn.executable("linthis") == 1 then
    print("[PASS] linthis executable found")
  else
    print("[WARN] linthis executable not found (install with: cargo install linthis)")
  end

  print("\n=== Tests Complete ===")
  vim.cmd("qa!")
end, 100)
EOF

# Create test file
cat > "$TEST_DIR/test.py" << 'EOF'
def hello():
    print("hello world")
EOF

# Run Neovim with test config
echo ""
echo "Running Neovim tests..."
echo ""

PLUGIN_DIR="$SCRIPT_DIR" XDG_CONFIG_HOME="$TEST_DIR" \
  nvim --headless -u "$TEST_DIR/nvim/init.lua" "$TEST_DIR/test.py" 2>&1

echo ""
echo "=== Done ==="
