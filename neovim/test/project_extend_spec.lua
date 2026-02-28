-- Project and Extend operator tests
-- Slice 7: project and extend operators

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

describe("KQL project and extend", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should parse project with column list without errors", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | project State, EventType")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(5000, function() return false end, 100)

    local diagnostics = vim.diagnostic.get(bufnr)
    assert.equals(0, #diagnostics, "project with columns should produce zero diagnostics")
  end)

  it("should parse extend with assignment without errors", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | extend Duration = EndTime - StartTime")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(5000, function() return false end, 100)

    local diagnostics = vim.diagnostic.get(bufnr)
    assert.equals(0, #diagnostics, "extend with assignment should produce zero diagnostics")
  end)

  it("should highlight 'project' as a keyword in semantic tokens", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | project State")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Request semantic tokens
    local result = nil
    local done = false
    client:request("textDocument/semanticTokens/full", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
    }, function(err, resp)
      if err then
        error("Semantic tokens request failed: " .. vim.inspect(err))
      end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get semantic tokens response")
    assert.is_not_nil(result.data, "Should have data array")

    -- Check that 'project' is highlighted as keyword (type index 0)
    -- Tokens: StormEvents(property=5), |(operator=4), project(keyword=0), State(property=5)
    local data = result.data
    assert.is_true(#data >= 20, "Should have at least 4 tokens (5 values each)")

    -- Third token (project) should have type 0 (keyword)
    -- Token at index 10-14: deltaLine, deltaStart, length, tokenType, tokenModifiers
    local project_token_type = data[14] -- 0-indexed: token[2].tokenType = data[13], but Lua is 1-indexed
    assert.equals(0, project_token_type, "project should be highlighted as keyword (type 0)")
  end)
end)
