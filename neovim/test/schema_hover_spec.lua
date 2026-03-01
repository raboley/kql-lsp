-- Schema hover tests: verify hover shows table columns and column types
-- Slice 25: Schema Hover (Tables and Columns)

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

describe("KQL schema hover", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should show column list when hovering over table name", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/hover", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 3 }, -- middle of "StormEvents"
    }, function(err, resp)
      if err then error("Hover failed: " .. vim.inspect(err)) end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a hover response for table name")

    local content = result.contents.value or result.contents
    assert(content:match("StormEvents"), "Hover should mention 'StormEvents'")
    assert(content:match("State"), "Hover should list 'State' column")
    assert(content:match("StartTime"), "Hover should list 'StartTime' column")
  end)

  it("should show column type when hovering over column name", function()
    local bufnr = helpers.open_kql_buffer('StormEvents | where State == "TX"')

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/hover", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 21 }, -- middle of "State"
    }, function(err, resp)
      if err then error("Hover failed: " .. vim.inspect(err)) end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a hover response for column name")

    local content = result.contents.value or result.contents
    assert(content:match("State"), "Hover should mention 'State'")
    assert(content:match("string"), "Hover should show type 'string'")
  end)
end)
