-- Test helpers for KQL LSP Neovim tests
local M = {}

--- Open a scratch buffer with .kql filetype and optional content.
--- Returns the buffer number.
---@param content string|nil Initial buffer content
---@return number bufnr
function M.open_kql_buffer(content)
  vim.cmd("enew")
  local bufnr = vim.api.nvim_get_current_buf()
  vim.bo[bufnr].filetype = "kql"
  vim.bo[bufnr].buftype = "nofile"
  -- Give it a .kql name so the LSP recognizes it
  vim.api.nvim_buf_set_name(bufnr, "test_" .. bufnr .. ".kql")

  if content then
    local lines = vim.split(content, "\n")
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  end

  return bufnr
end

--- Wait for an LSP client named "kql-lsp" to attach to the buffer.
--- Returns the client or nil on timeout.
---@param bufnr number
---@param timeout_ms number|nil Default 10000
---@return table|nil client
function M.wait_for_client(bufnr, timeout_ms)
  timeout_ms = timeout_ms or 10000
  local client = nil

  vim.wait(timeout_ms, function()
    local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "kql-lsp" })
    if #clients > 0 then
      client = clients[1]
      return true
    end
    return false
  end, 100)

  return client
end

--- Wait for diagnostics to appear on the buffer.
--- Returns the diagnostics list.
---@param bufnr number
---@param timeout_ms number|nil Default 10000
---@return table diagnostics
function M.wait_for_diagnostics(bufnr, timeout_ms)
  timeout_ms = timeout_ms or 10000
  local diagnostics = {}

  vim.wait(timeout_ms, function()
    diagnostics = vim.diagnostic.get(bufnr)
    return #diagnostics > 0
  end, 100)

  return diagnostics
end

--- Stop all kql-lsp clients and clean up buffers.
function M.cleanup()
  -- Stop all kql-lsp clients
  local clients = vim.lsp.get_clients({ name = "kql-lsp" })
  for _, client in ipairs(clients) do
    client:stop(true)
  end

  -- Wait briefly for clients to shut down
  vim.wait(2000, function()
    return #vim.lsp.get_clients({ name = "kql-lsp" }) == 0
  end, 100)

  -- Wipe all buffers
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(bufnr) then
      pcall(vim.api.nvim_buf_delete, bufnr, { force = true })
    end
  end
end

return M
