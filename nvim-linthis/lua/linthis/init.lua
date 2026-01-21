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

-- Start LSP client for buffer
local function start_lsp(bufnr)
  local opts = config.get()
  local fname = vim.api.nvim_buf_get_name(bufnr)

  -- Check if already attached
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "linthis" })
  if #clients > 0 then
    return clients[1]
  end

  -- Start new client
  local client_id = vim.lsp.start({
    name = "linthis",
    cmd = opts.cmd,
    root_dir = find_root(fname),
    capabilities = vim.lsp.protocol.make_client_capabilities(),
  }, {
    bufnr = bufnr,
  })

  return client_id
end

-- Setup LSP autocommand
local function setup_lsp()
  local opts = config.get()
  local group = vim.api.nvim_create_augroup("linthis_lsp", { clear = true })

  vim.api.nvim_create_autocmd("FileType", {
    group = group,
    pattern = opts.filetypes,
    callback = function(args)
      if opts.autostart then
        start_lsp(args.buf)
      end
    end,
  })
end

-- Format current buffer using linthis CLI
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

-- Lint current buffer using linthis CLI
function M.lint(opts)
  opts = opts or {}
  local bufnr = opts.bufnr or vim.api.nvim_get_current_buf()
  local filepath = vim.api.nvim_buf_get_name(bufnr)

  if filepath == "" then
    if config.get().notifications then
      vim.notify("linthis: cannot lint unsaved buffer", vim.log.levels.WARN)
    end
    return
  end

  -- Save buffer first if modified
  if vim.bo[bufnr].modified then
    vim.api.nvim_buf_call(bufnr, function()
      vim.cmd("silent write")
    end)
  end

  -- Run linthis -c (check only, no format)
  local cmd = config.get().cmd[1]
  local result = vim.fn.system({ cmd, "-c", "-i", filepath })

  -- Parse output and set diagnostics
  local diagnostics = {}
  local ns = vim.api.nvim_create_namespace("linthis")

  -- Parse linthis output format: [E1][lang][tool] file:line:col: severity: message (code)
  -- Example: [E1][python][ruff] /tmp/test.py:1:8: error: `os` imported but unused (F401)
  for line in result:gmatch("[^\r\n]+") do
    -- Match: [idx][lang][tool] file:line:col: severity: message
    local tool, file, lnum, col, severity, msg =
      line:match("^%[%w+%]%[%w+%]%[(%w+)%]%s+(.+):(%d+):(%d+):%s*(%w+):%s*(.+)$")

    if tool and lnum and msg then
      local sev = vim.diagnostic.severity.WARN
      if severity == "error" then
        sev = vim.diagnostic.severity.ERROR
      elseif severity == "hint" or severity == "info" then
        sev = vim.diagnostic.severity.HINT
      end

      -- Extract code from message if present (e.g., "message (F401)")
      local message, code = msg:match("^(.+)%s+%(([^)]+)%)$")
      if not message then
        message = msg
      end

      table.insert(diagnostics, {
        lnum = tonumber(lnum) - 1,
        col = tonumber(col) - 1,
        message = message,
        severity = sev,
        source = "linthis-" .. tool, -- e.g., "linthis-ruff"
        code = code,
      })
    end
  end

  -- Set diagnostics
  vim.diagnostic.set(ns, bufnr, diagnostics)

  if config.get().notifications and not opts.silent then
    local count = #diagnostics
    if count > 0 then
      vim.notify(string.format("linthis: %d issue(s) found", count), vim.log.levels.INFO)
    else
      vim.notify("linthis: no issues found", vim.log.levels.INFO)
    end
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
    start_lsp(bufnr)
    if config.get().notifications then
      vim.notify("linthis: LSP restarted", vim.log.levels.INFO)
    end
  end, 500)
end

-- Get LSP info
function M.info()
  local bufnr = vim.api.nvim_get_current_buf()
  local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "linthis" })
  local cmd = config.get().cmd[1]

  print("linthis info:")
  print(string.format("  Executable: %s", cmd))
  print(string.format("  Executable found: %s", vim.fn.executable(cmd) == 1 and "yes" or "no"))

  if #clients == 0 then
    print("  LSP: not attached")
  else
    for _, client in ipairs(clients) do
      print(string.format("  LSP Client ID: %d", client.id))
      print(string.format("  LSP Root: %s", client.config.root_dir or "none"))
    end
  end
end

-- Setup autocommands
local function setup_autocmds()
  local opts = config.get()
  local group = vim.api.nvim_create_augroup("linthis", { clear = true })

  -- Format on save
  if opts.format_on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = group,
      pattern = "*",
      callback = function(args)
        local ft = vim.bo[args.buf].filetype
        if vim.tbl_contains(opts.filetypes, ft) then
          local filepath = vim.api.nvim_buf_get_name(args.buf)
          local cmd = config.get().cmd[1]
          vim.fn.system({ cmd, "-f", "-i", filepath })
          if vim.v.shell_error == 0 then
            vim.api.nvim_buf_call(args.buf, function()
              vim.cmd("silent edit!")
            end)
          end
        end
      end,
    })
  end

  -- Lint on save
  if opts.lint_on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = group,
      pattern = "*",
      callback = function(args)
        local ft = vim.bo[args.buf].filetype
        if vim.tbl_contains(opts.filetypes, ft) then
          M.lint({ bufnr = args.buf, silent = true })
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
  end, { desc = "Show linthis info" })
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
