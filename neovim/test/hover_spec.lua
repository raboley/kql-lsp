-- Hover tests: verify hover documentation for built-in functions
-- Slice 9: Hover for Built-in Functions

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

describe("KQL hover", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should return hover documentation for count()", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | summarize count()")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Hover over "count" (position at character 24, which is inside "count")
    local result = nil
    local done = false
    client:request("textDocument/hover", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 24 }, -- inside "count"
    }, function(err, resp)
      if err then
        error("Hover request failed: " .. vim.inspect(err))
      end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a hover response")
    assert.is_not_nil(result.contents, "Hover should have contents")

    -- Contents should mention count
    local value = result.contents.value or result.contents
    assert.is_truthy(value:find("count"), "Hover should mention 'count'")
  end)

  it("should return null for unknown identifiers", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Hover over "StormEvents" (table name, no documentation)
    local result = nil
    local done = false
    client:request("textDocument/hover", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 3 }, -- inside "StormEvents"
    }, function(err, resp)
      if err then
        error("Hover request failed: " .. vim.inspect(err))
      end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    -- Result should be nil/null for unknown identifiers
    assert.is_nil(result, "Should get null for unknown identifiers")
  end)
end)
