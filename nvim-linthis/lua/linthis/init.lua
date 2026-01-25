-- linthis.nvim - Neovim plugin for linthis
-- Multi-language linter and formatter

local M = {}
local config = require("linthis.config")

-- Compatibility layer for Neovim 0.9.x and 0.10+
-- vim.lsp.get_clients() was added in 0.10, use get_active_clients() for 0.9.x
local get_clients
if vim.lsp.get_clients then
  get_clients = vim.lsp.get_clients
elseif vim.lsp.get_active_clients then
  get_clients = function(opts)
    -- get_active_clients in 0.9.x supports {bufnr=bufnr} but we need to filter by name manually
    local clients = vim.lsp.get_active_clients(opts)
    if opts and opts.name then
      local filtered = {}
      for _, client in ipairs(clients) do
        if client.name == opts.name then
          table.insert(filtered, client)
        end
      end
      return filtered
    end
    return clients
  end
else
  -- Fallback for very old versions
  get_clients = function() return {} end
end

-- vim.fs.root was added in 0.10, provide fallback for 0.9
local function find_root_dir(fname, markers)
  if vim.fs.root then
    return vim.fs.root(fname, markers)
  end
  -- Fallback for Neovim 0.9.x
  local path = vim.fn.fnamemodify(fname, ":p:h")
  while path and path ~= "/" do
    for _, marker in ipairs(markers) do
      local marker_path = path .. "/" .. marker
      if vim.fn.filereadable(marker_path) == 1 or vim.fn.isdirectory(marker_path) == 1 then
        return path
      end
    end
    path = vim.fn.fnamemodify(path, ":h")
  end
  return nil
end

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

  local root = find_root_dir(fname, markers)
  return root or vim.fn.getcwd()
end

-- Start LSP client for buffer
local function start_lsp(bufnr)
  local opts = config.get()
  local fname = vim.api.nvim_buf_get_name(bufnr)

  -- Check if already attached
  local clients = get_clients({ bufnr = bufnr, name = "linthis" })
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

  -- Get or create linthis namespace
  local ns = vim.api.nvim_create_namespace("linthis")

  -- Clear previous linthis diagnostics for this buffer first
  vim.diagnostic.set(ns, bufnr, {})

  -- Run linthis -c (check only, no format)
  -- Use --no-cache to ensure fresh results on each lint
  local cmd = config.get().cmd[1]
  local result = vim.fn.system({ cmd, "-c", "--no-cache", "-i", filepath })

  -- Strip ANSI escape codes from output (comprehensive)
  result = result:gsub("\27%[[%d;]*[mKHJGsu]", "")  -- Common SGR and cursor codes
  result = result:gsub("\27%[%?%d+[hl]", "")        -- DEC private modes
  result = result:gsub("\27%[[%d;]*[ABCDEFG]", "")  -- Cursor movement
  result = result:gsub("\r", "")                     -- Carriage returns

  -- Parse output and set diagnostics
  local diagnostics = {}

  -- Parse linthis output format: [E1][lang][tool] file:line:col: severity: message (code)
  -- Example: [E1][python][ruff] /tmp/test.py:1:8: error: `os` imported but unused (F401)
  --   --> Remove unused import: `os`
  local lines = {}
  for line in result:gmatch("[^\r\n]+") do
    table.insert(lines, line)
  end

  local i = 1
  while i <= #lines do
    local line = lines[i]
    -- Match: [idx][lang][tool] file:line:col: severity: message
    local tool, file, lnum, col, severity, msg =
      line:match("^%[E?%d+%]%[%w+%]%[(%w+)%]%s+(.+):(%d+):(%d+):%s*(%w+):%s*(.+)$")

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

      -- Check next line for suggestion (starts with "  --> ")
      local suggestion = nil
      if i + 1 <= #lines then
        local next_line = lines[i + 1]
        local sugg = next_line:match("^%s*%-%->%s*(.+)$")
        if sugg then
          suggestion = sugg
          i = i + 1 -- Skip the suggestion line
        end
      end

      -- Include source prefix directly in message for reliable display
      local source_name = "linthis-" .. tool
      local display_message = string.format("[%s] %s", source_name, message)
      if suggestion then
        display_message = display_message .. " | Suggestion: " .. suggestion
      end

      table.insert(diagnostics, {
        lnum = tonumber(lnum) - 1,
        col = tonumber(col) - 1,
        message = display_message,
        severity = sev,
        source = source_name,
        code = code,
      })
    end
    i = i + 1
  end

  -- Set linthis diagnostics (this replaces any existing linthis diagnostics for this buffer)
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
  local clients = get_clients({ bufnr = bufnr, name = "linthis" })

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

-- Debug format output
function M.debug_format()
  local bufnr = vim.api.nvim_get_current_buf()
  local filepath = vim.api.nvim_buf_get_name(bufnr)

  if filepath == "" then
    print("linthis debug: no file")
    return
  end

  -- Save buffer first if modified
  if vim.bo[bufnr].modified then
    vim.api.nvim_buf_call(bufnr, function()
      vim.cmd("silent write")
    end)
  end

  local cmd = config.get().cmd[1]
  print("=== Debug Format ===")
  print("Command: " .. cmd)
  print("File: " .. filepath)
  print("Full command: " .. cmd .. " -f -i " .. filepath)
  print("=== Running format ===")

  local result = vim.fn.system({ cmd, "-f", "-i", filepath })
  local exit_code = vim.v.shell_error

  print("Exit code: " .. exit_code)
  print("Output: " .. result)
  print("=== Done ===")

  -- Show file content after format
  print("=== File content after format (first 10 lines) ===")
  local lines = vim.fn.readfile(filepath, "", 10)
  for i, line in ipairs(lines) do
    print(i .. ": " .. line)
  end
end

-- Debug lint output
function M.debug_lint()
  local bufnr = vim.api.nvim_get_current_buf()
  local filepath = vim.api.nvim_buf_get_name(bufnr)

  if filepath == "" then
    print("linthis debug: no file")
    return
  end

  local cmd = config.get().cmd[1]
  local result = vim.fn.system({ cmd, "-c", "--no-cache", "-i", filepath })

  print("=== Raw output ===")
  print(result)
  print("=== End raw output ===")

  -- Strip ANSI
  result = result:gsub("\27%[[%d;]*[mKHJ]", "")
  result = result:gsub("\27%[%?%d+[hl]", "")

  print("=== Parsed lines ===")
  local lines = {}
  for line in result:gmatch("[^\r\n]+") do
    table.insert(lines, line)
    print(string.format("Line %d: [%s]", #lines, line))
  end

  print("=== Matching ===")
  local i = 1
  while i <= #lines do
    local line = lines[i]
    local tool, file, lnum, col, severity, msg =
      line:match("^%[E?%d+%]%[%w+%]%[(%w+)%]%s+(.+):(%d+):(%d+):%s*(%w+):%s*(.+)$")

    if tool then
      print(string.format("Issue: tool=%s, msg=%s", tool, msg))

      -- Check next line for suggestion
      if i + 1 <= #lines then
        local next_line = lines[i + 1]
        print(string.format("  Next line: [%s]", next_line))
        local sugg = next_line:match("^%s*%-%->%s*(.+)$")
        if sugg then
          print(string.format("  Suggestion found: %s", sugg))
          i = i + 1
        else
          print("  No suggestion match")
        end
      end
    end
    i = i + 1
  end
  print("=== Done ===")
end

-- Quick test function to verify lint parsing works
function M.test()
  local bufnr = vim.api.nvim_get_current_buf()
  local filepath = vim.api.nvim_buf_get_name(bufnr)

  print("=== linthis test ===")
  print("File: " .. (filepath ~= "" and filepath or "(unsaved)"))
  print("Filetype: " .. vim.bo[bufnr].filetype)

  if filepath == "" then
    print("ERROR: Cannot test unsaved buffer")
    return
  end

  -- Run linthis
  local cmd = config.get().cmd[1]
  print("Command: " .. cmd .. " -c --no-cache -i " .. filepath)

  local result = vim.fn.system({ cmd, "-c", "--no-cache", "-i", filepath })
  local exit_code = vim.v.shell_error

  print("Exit code: " .. exit_code)
  print("Output length: " .. #result .. " bytes")

  -- Strip ANSI
  result = result:gsub("\27%[[%d;]*[mKHJGsu]", "")
  result = result:gsub("\27%[%?%d+[hl]", "")
  result = result:gsub("\27%[[%d;]*[ABCDEFG]", "")
  result = result:gsub("\r", "")

  -- Count diagnostics
  local count = 0
  for line in result:gmatch("[^\n]+") do
    if line:match("^%[E?%d+%]%[%w+%]%[%w+%]") then
      count = count + 1
      print("Found: " .. line:sub(1, 100))
    end
  end

  print("Diagnostics found: " .. count)

  -- Run actual lint
  M.lint({ silent = false })

  -- Check diagnostics
  local ns = vim.api.nvim_create_namespace("linthis")
  local diags = vim.diagnostic.get(bufnr, { namespace = ns })
  print("Diagnostics set: " .. #diags)

  for i, d in ipairs(diags) do
    print(string.format("  [%d] line %d: %s", i, d.lnum + 1, d.message:sub(1, 60)))
  end
  print("=== end test ===")
end

-- Show all diagnostics for current line (useful when multiple sources)
function M.show_line_diagnostics()
  local bufnr = vim.api.nvim_get_current_buf()
  local line = vim.api.nvim_win_get_cursor(0)[1] - 1

  -- Get all diagnostics for this line
  local all_diags = vim.diagnostic.get(bufnr, { lnum = line })

  if #all_diags == 0 then
    print("No diagnostics on this line")
    return
  end

  print(string.format("=== %d diagnostics on line %d ===", #all_diags, line + 1))
  for i, d in ipairs(all_diags) do
    local source = d.source or "unknown"
    local code = d.code and string.format(" (%s)", d.code) or ""
    print(string.format("[%d] [%s]%s: %s", i, source, code, d.message))
  end
end

-- Get LSP info
function M.info()
  local bufnr = vim.api.nvim_get_current_buf()
  local clients = get_clients({ bufnr = bufnr, name = "linthis" })
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
          if vim.api.nvim_buf_is_valid(args.buf) then
            M.lint({ bufnr = args.buf, silent = true })
          end
        end
      end,
    })
  end

  -- Lint on open
  if opts.lint_on_open then
    vim.api.nvim_create_autocmd("FileType", {
      group = group,
      pattern = opts.filetypes,
      callback = function(args)
        -- Delay to let other LSPs attach and report diagnostics first
        -- Then run linthis lint to show our diagnostics
        vim.defer_fn(function()
          if vim.api.nvim_buf_is_valid(args.buf) then
            local filepath = vim.api.nvim_buf_get_name(args.buf)
            -- Only lint if file exists on disk
            if filepath ~= "" and vim.fn.filereadable(filepath) == 1 then
              M.lint({ bufnr = args.buf, silent = true })
            end
          end
        end, 1000)
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

  vim.api.nvim_create_user_command("LinthisDebug", function()
    M.debug_lint()
  end, { desc = "Debug linthis lint output" })

  vim.api.nvim_create_user_command("LinthisDebugFormat", function()
    M.debug_format()
  end, { desc = "Debug linthis format output" })

  vim.api.nvim_create_user_command("LinthisTest", function()
    M.test()
  end, { desc = "Quick test to verify linthis lint works" })

  vim.api.nvim_create_user_command("LinthisShowDiagnostics", function()
    M.show_line_diagnostics()
  end, { desc = "Show all diagnostics on current line from all sources" })
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
