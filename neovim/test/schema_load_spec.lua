-- Schema loading tests: verify LSP accepts schema configuration
-- Slice 21: Config + ADX Live Schema Fetching + Schema Types

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

-- Set init_options with schemaFile BEFORE sourcing the plugin
local schema_path = project_root .. "/neovim/test-data/.kql-schema.json"
vim.g.kql_lsp_init_options = { schemaFile = schema_path }
vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe("KQL schema loading", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should complete table names at start of query when schema is loaded", function()
    local bufnr = helpers.open_kql_buffer("Storm")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Request completion at end of "Storm" (start of query, not after pipe)
    local result = nil
    local done = false
    client:request("textDocument/completion", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 5 },
    }, function(err, resp)
      if err then error("Completion failed: " .. vim.inspect(err)) end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a completion response")

    local items = result.items or result
    local labels = {}
    for _, item in ipairs(items) do
      labels[item.label] = true
    end

    assert.is_true(labels["StormEvents"] ~= nil, "Should include 'StormEvents' table from schema")
    assert.is_true(labels["PopulationData"] ~= nil, "Should include 'PopulationData' table from schema")
  end)

  it("should start LSP with missing schemaFile without crashing", function()
    -- Override init_options with a non-existent file
    vim.g.kql_lsp_init_options = { schemaFile = "/tmp/nonexistent-schema.json" }

    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach even with bad schema path")

    vim.wait(3000, function() return false end, 100)

    local clients = vim.lsp.get_clients({ bufnr = bufnr, name = "kql-lsp" })
    assert.is_true(#clients > 0, "LSP should not crash on missing schema file")
  end)
end)
