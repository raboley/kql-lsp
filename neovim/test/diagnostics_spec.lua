-- Diagnostics tests: verify parser produces correct error diagnostics
-- Slice 2: Parse simple query + first real diagnostic

local this_file = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/")
local test_dir = vim.fn.fnamemodify(this_file, ":h")
local neovim_dir = vim.fn.fnamemodify(test_dir, ":h")
local project_root = vim.fn.fnamemodify(neovim_dir, ":h")

local helpers = dofile(test_dir .. "/helpers.lua")

-- Determine binary path
local binary = project_root .. "/lsp/target/release/kql-lsp.exe"
if vim.fn.has("unix") == 1 then
  binary = project_root .. "/lsp/target/release/kql-lsp"
end

-- Source the plugin
vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe("KQL diagnostics", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should produce error diagnostic for incomplete where clause", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | where")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait for diagnostics with errors
    local diagnostics = helpers.wait_for_diagnostics(bufnr, 10000)
    assert.is_true(#diagnostics > 0, "Should produce at least one diagnostic for incomplete 'where'")

    -- Verify at least one is an error
    local has_error = false
    for _, d in ipairs(diagnostics) do
      if d.severity == vim.diagnostic.severity.ERROR then
        has_error = true
        -- Verify the diagnostic is near 'where' keyword (col ~14-19), not at end of file
        -- 'StormEvents | where' : 'where' ends at col 19
        assert.is_true(d.col <= 19, "Diagnostic should be near 'where' keyword, not at end of file (col=" .. d.col .. ")")
        break
      end
    end
    assert.is_true(has_error, "Should have at least one ERROR severity diagnostic")
  end)

  it("should produce zero diagnostics for valid query", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait a reasonable time for diagnostics to arrive
    vim.wait(5000, function() return false end, 100)

    local diagnostics = vim.diagnostic.get(bufnr)
    assert.equals(0, #diagnostics, "Valid query should produce zero diagnostics")
  end)

  it("should clear diagnostics when editing from invalid to valid", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | where")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait for error diagnostics to appear
    local diagnostics = helpers.wait_for_diagnostics(bufnr, 10000)
    assert.is_true(#diagnostics > 0, "Should have diagnostics for invalid query")

    -- Fix the query by making it valid
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, { "StormEvents | take 10" })

    -- Wait for diagnostics to clear
    vim.wait(5000, function()
      return #vim.diagnostic.get(bufnr) == 0
    end, 100)

    diagnostics = vim.diagnostic.get(bufnr)
    assert.equals(0, #diagnostics, "Diagnostics should clear after fixing the query")
  end)
end)
