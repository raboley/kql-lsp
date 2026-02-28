-- Completion tests: verify pipe operator completions
-- Slice 5: Completion After Pipe

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

describe("KQL completion", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should return tabular operator completions after pipe", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | ")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)

    -- Request completion at end of line (after "| ")
    local result = nil
    local done = false
    client:request("textDocument/completion", {
      textDocument = { uri = vim.uri_from_bufnr(bufnr) },
      position = { line = 0, character = 14 }, -- after "StormEvents | "
    }, function(err, resp)
      if err then
        error("Completion request failed: " .. vim.inspect(err))
      end
      result = resp
      done = true
    end, bufnr)

    vim.wait(10000, function() return done end, 100)
    assert.is_not_nil(result, "Should get a completion response")

    -- Result can be CompletionList or CompletionItem[]
    local items = result
    if result.items then
      items = result.items
    end

    assert.is_true(#items >= 4, "Should have at least 4 operator completions, got " .. #items)

    -- Collect completion labels
    local labels = {}
    for _, item in ipairs(items) do
      labels[item.label] = true
    end

    assert.is_true(labels["where"] ~= nil, "Should include 'where' completion")
    assert.is_true(labels["project"] ~= nil, "Should include 'project' completion")
    assert.is_true(labels["summarize"] ~= nil, "Should include 'summarize' completion")
    assert.is_true(labels["take"] ~= nil, "Should include 'take' completion")
  end)
end)
