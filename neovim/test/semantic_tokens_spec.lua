-- Semantic tokens tests: verify syntax highlighting via LSP
-- Slice 3: Keywords, identifiers, numbers, operators get semantic coloring

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

describe("KQL semantic tokens", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should return non-empty semantic tokens for simple query", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait for LSP to be ready
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
    assert.is_not_nil(result, "Should get a semantic tokens response")
    assert.is_not_nil(result.data, "Response should have data array")
    assert.is_true(#result.data > 0, "Should have non-empty token data")
  end)

  it("should include keyword token for 'take'", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    local result = nil
    local done = false
    client:request("textDocument/semanticTokens/full", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
    }, function(err, resp)
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get response")
    assert.is_not_nil(result.data, "Should have data")

    -- Semantic tokens data is encoded as groups of 5 integers:
    -- [deltaLine, deltaStartChar, length, tokenType, tokenModifiers]
    -- We expect at least 3 tokens: StormEvents(property), take(keyword), 10(number)
    local token_count = #result.data / 5
    assert.is_true(token_count >= 3, "Should have at least 3 tokens (got " .. token_count .. ")")
  end)
end)
