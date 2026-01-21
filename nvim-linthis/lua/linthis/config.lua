-- linthis.nvim configuration
local M = {}

-- Default configuration
M.defaults = {
  -- Path to linthis executable
  cmd = { "linthis", "lsp" },

  -- Filetypes to attach to
  filetypes = {
    "rust",
    "python",
    "typescript",
    "javascript",
    "typescriptreact",
    "javascriptreact",
    "go",
    "java",
    "c",
    "cpp",
    "objc",
    "swift",
    "kotlin",
    "lua",
    "dart",
    "sh",
    "bash",
    "zsh",
    "ruby",
    "php",
    "scala",
    "cs",
  },

  -- Root directory patterns
  root_markers = {
    ".linthis.toml",
    ".git",
    "Cargo.toml",
    "pyproject.toml",
    "package.json",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "Makefile",
  },

  -- Auto-start LSP when opening supported files
  autostart = true,

  -- Format on save
  format_on_save = false,

  -- Lint on save (trigger diagnostics refresh)
  lint_on_save = true,

  -- Lint on open (run lint when opening a file)
  lint_on_open = true,

  -- Show notifications
  notifications = true,

  -- Log level: "debug", "info", "warn", "error"
  log_level = "warn",
}

-- Current configuration (will be merged with user config)
M.options = {}

-- Setup configuration
function M.setup(opts)
  M.options = vim.tbl_deep_extend("force", {}, M.defaults, opts or {})
end

-- Get current configuration
function M.get()
  if vim.tbl_isempty(M.options) then
    M.setup({})
  end
  return M.options
end

return M
