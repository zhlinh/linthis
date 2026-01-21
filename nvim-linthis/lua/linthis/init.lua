-- linthis.nvim - Neovim plugin for linthis
-- Multi-language linter and formatter

local M = {}
local config = require("linthis.config")

-- Check if linthis executable exists
local function check_executable()
  local cmd = config.get().cmd[1]
  if vim.fn.executable(cmd) == 0 then
    if config.get().notifications then
      vim.notify(
        string.format("linthis: executable '%s' not found. Please install linthis.", cmd),
        vim.log.levels.ERROR
      )
    end
    return false
  end
  return true
end

-- Find root directory
local function find_root(fname)
  local markers = config.get().root_markers
  if not fname or fname == "" then
    return vim.fn.getcwd()
  end

  local root = vim.fs.root(fname, markers)
  return root or vim.fn.getcwd()
end

-- Setup LSP client
local function setup_lsp()
  local opts = config.get()

  -- Create LSP client configuration
  local lsp_config = {
    name = "linthis",
    cmd = opts.cmd,
    filetypes = opts.filetypes,
    root_dir = function(fname)
      return find_root(fname)
    end,
    settings = {},
    init_options = {},
    capabilities = vim.lsp.protocol.make_client_capabilities(),
  }

  -- Try to enhance capabilities with cmp-nvim-lsp if available
  local ok, cmp_lsp = pcall(require, "cmp_nvim_lsp")
  if ok then
    lsp_config.capabilities = cmp_lsp.default_capabilities(lsp_config.capabilities)
  end

  -- Register the LSP configuration
  vim.lsp.config.linthis = lsp_config

  -- Enable the LSP for configured filetypes
  vim.lsp.enable("linthis")
end

-- Format current buffer using linthis CLI (like VSCode plugin)
function M.format(opts)
  opts = opts or {}
  local bufnr = opts.bufnr or vim.api.nvim_get_current_buf()
  local filepath = vim.api.nvim_buf_get_name(bufnr)

  if filepath == "" then
    if config.get().notifications then
      vim.notify("linthis: cannot format unsaved buffer", vim.log.levels.WARN)
    end
    return false
  end

  -- Save buffer first if modified
  if vim.bo[bufnr].modified then
    vim.api.nvim_buf_call(bufnr, function()
      vim.cmd("silent write")
    end)
  end

  -- Run linthis -f -i (format in-place)
  local cmd = config.get().cmd[1]
  local result = vim.fn.system({ cmd, "-f", "-i", filepath })
  local exit_code = vim.v.shell_error

  if exit_code == 0 then
    -- Reload buffer to show formatted content
    vim.api.nvim_buf_call(bufnr, function()
      vim.cmd("silent edit!")
    end)

    if config.get().notifications and not opts.silent then
      vim.notify("linthis: formatted", vim.log.levels.INFO)
    end
    return true
  else
    if config.get().notifications and not opts.silent then
      vim.notify("linthis: format failed - " .. vim.trim(result), vim.log.levels.ERROR)
    end
    return false
  end
end

-- Lint current buffer (refresh diagnostics)
function M.lint(opts)
  opts = opts or {}
  local bufnr = opts.bufnr or vim.api.nvim_get_current_buf()

  -- Force diagnostic refresh by sending didChange notification
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "linthis" })
  if #clients == 0 then
    if config.get().notifications then
      vim.notify("linthis: LSP not attached to this buffer", vim.log.levels.WARN)
    end
    return
  end

  -- Trigger diagnostics refresh
  vim.diagnostic.reset(nil, bufnr)
  for _, client in ipairs(clients) do
    local params = vim.lsp.util.make_text_document_params(bufnr)
    client:notify("textDocument/didSave", params)
  end

  if config.get().notifications and not opts.silent then
    vim.schedule(function()
      local diagnostics = vim.diagnostic.get(bufnr)
      local count = #diagnostics
      if count > 0 then
        vim.notify(string.format("linthis: %d issue(s) found", count), vim.log.levels.INFO)
      else
        vim.notify("linthis: no issues found", vim.log.levels.INFO)
      end
    end)
  end
end

-- Restart LSP server
function M.restart()
  local bufnr = vim.api.nvim_get_current_buf()
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "linthis" })

  for _, client in ipairs(clients) do
    vim.lsp.stop_client(client.id)
  end

  -- Restart after a short delay
  vim.defer_fn(function()
    vim.cmd("edit")
    if config.get().notifications then
      vim.notify("linthis: LSP restarted", vim.log.levels.INFO)
    end
  end, 500)
end

-- Get LSP info
function M.info()
  local bufnr = vim.api.nvim_get_current_buf()
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "linthis" })

  if #clients == 0 then
    print("linthis: LSP not attached")
    return
  end

  for _, client in ipairs(clients) do
    print(string.format("linthis LSP:"))
    print(string.format("  Client ID: %d", client.id))
    print(string.format("  Root: %s", client.config.root_dir or "none"))
    print(string.format("  Cmd: %s", table.concat(client.config.cmd, " ")))
  end
end

-- Setup autocommands
local function setup_autocmds()
  local opts = config.get()
  local group = vim.api.nvim_create_augroup("linthis", { clear = true })

  -- Format on save (use BufWritePost to avoid conflicts)
  if opts.format_on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = group,
      pattern = "*",
      callback = function(args)
        local ft = vim.bo[args.buf].filetype
        if vim.tbl_contains(opts.filetypes, ft) then
          -- Run format after save, then reload
          local filepath = vim.api.nvim_buf_get_name(args.buf)
          local cmd = config.get().cmd[1]
          local result = vim.fn.system({ cmd, "-f", "-i", filepath })
          local exit_code = vim.v.shell_error

          if exit_code == 0 then
            -- Reload buffer to show formatted content
            vim.api.nvim_buf_call(args.buf, function()
              vim.cmd("silent edit!")
            end)
          end
        end
      end,
    })
  end
end

-- Setup user commands
local function setup_commands()
  vim.api.nvim_create_user_command("LinthisFormat", function()
    M.format()
  end, { desc = "Format current buffer with linthis" })

  vim.api.nvim_create_user_command("LinthisLint", function()
    M.lint()
  end, { desc = "Lint current buffer with linthis" })

  vim.api.nvim_create_user_command("LinthisRestart", function()
    M.restart()
  end, { desc = "Restart linthis LSP server" })

  vim.api.nvim_create_user_command("LinthisInfo", function()
    M.info()
  end, { desc = "Show linthis LSP info" })
end

-- Main setup function
function M.setup(opts)
  -- Merge configuration
  config.setup(opts)

  -- Check executable
  if not check_executable() then
    return
  end

  -- Setup LSP
  setup_lsp()

  -- Setup autocommands
  setup_autocmds()

  -- Setup user commands
  setup_commands()

  if config.get().notifications then
    vim.notify("linthis: initialized", vim.log.levels.DEBUG)
  end
end

return M
