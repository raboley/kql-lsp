-- Document symbols tests: verify outline view shows let bindings and queries
-- Slice 4: Document Symbols for Let + Query

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

describe("KQL document symbols", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should return symbols for let binding and query", function()
    local bufnr = helpers.open_kql_buffer("let threshold = 100;\nStormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Request document symbols
    local result = nil
    local done = false
    client:request("textDocument/documentSymbol", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
    }, function(err, resp)
      if err then
        error("Document symbols request failed: " .. vim.inspect(err))
      end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a document symbols response")
    assert.equals(2, #result, "Should have 2 symbols (let + query)")

    -- Find the let symbol
    local let_symbol = nil
    local query_symbol = nil
    for _, sym in ipairs(result) do
      if sym.name == "threshold" then
        let_symbol = sym
      elseif sym.name == "StormEvents" then
        query_symbol = sym
      end
    end

    assert.is_not_nil(let_symbol, "Should have a 'threshold' symbol")
    assert.is_not_nil(query_symbol, "Should have a 'StormEvents' symbol")
    -- SymbolKind.Variable = 13
    assert.equals(13, let_symbol.kind, "Let binding should be Variable kind")
  end)
end)
