-- Column completion tests: verify LSP returns column names in context
-- Slice 23: Column Name Completion (Context-Aware)

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

describe("KQL column completion", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should complete column names after where operator", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | where ")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/completion", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 20 },
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

    assert.is_true(labels["State"] ~= nil, "Should include 'State' column from StormEvents")
    assert.is_true(labels["StartTime"] ~= nil, "Should include 'StartTime' column from StormEvents")
    assert.is_true(labels["DamageProperty"] ~= nil, "Should include 'DamageProperty' column from StormEvents")
  end)

  it("should complete column names after project operator", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | project ")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/completion", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 22 },
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

    assert.is_true(labels["State"] ~= nil, "Should include 'State' column after project")
    assert.is_true(labels["EventType"] ~= nil, "Should include 'EventType' column after project")
  end)

  it("should not complete columns for unknown tables", function()
    local bufnr = helpers.open_kql_buffer("UnknownTable | where ")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/completion", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 21 },
    }, function(err, resp)
      if err then error("Completion failed: " .. vim.inspect(err)) end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)

    local items = (result and (result.items or result)) or {}
    local labels = {}
    for _, item in ipairs(items) do
      labels[item.label] = true
    end

    -- Should get tabular operators (after pipe context), not column names
    assert.is_nil(labels["State"], "Should NOT include column names for unknown table")
    assert.is_nil(labels["StartTime"], "Should NOT include column names for unknown table")
  end)
end)
