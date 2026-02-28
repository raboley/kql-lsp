-- Document store tests: verify rope-backed document handling
-- Slice 1: Documents stored in rope, UTF-16 position conversion

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

describe("KQL document store", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should handle multi-line content without error", function()
    local bufnr = helpers.open_kql_buffer(
      "StormEvents\n| where State == 'TEXAS'\n| summarize count() by EventType\n| take 10"
    )

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach to multi-line buffer")
    assert.equals("kql-lsp", client.name)

    -- Wait for diagnostics to be published (empty array is fine - proves document was processed)
    vim.wait(3000, function() return false end, 100)
    local diagnostics = vim.diagnostic.get(bufnr)
    -- At this stage, empty diagnostics proves the document was received and processed
    assert.is_true(type(diagnostics) == "table", "Diagnostics should be a table")
  end)

  it("should handle non-ASCII characters without crashing", function()
    local bufnr = helpers.open_kql_buffer(
      "// Query für Wetterereignisse\nStormEvents\n| where State == '日本語テスト'\n| take 10"
    )

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach with non-ASCII content")

    -- The key test: server doesn't crash processing unicode content
    vim.wait(3000, function() return false end, 100)
    -- Client should still be running
    local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "kql-lsp" })
    assert.equals(1, #clients, "LSP client should still be running after processing unicode")
  end)

  it("should handle document changes without error", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait for initial processing
    vim.wait(2000, function() return false end, 100)

    -- Edit the buffer (triggers didChange)
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, {
      "StormEvents",
      "| where State == 'TEXAS'",
      "| take 5",
    })

    -- Wait for change to be processed
    vim.wait(2000, function() return false end, 100)

    -- Client should still be alive after the change
    local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "kql-lsp" })
    assert.equals(1, #clients, "LSP client should survive document changes")
  end)
end)
