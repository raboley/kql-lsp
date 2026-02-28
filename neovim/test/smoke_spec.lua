-- Smoke test: verify KQL LSP starts and attaches to a .kql buffer

-- Compute paths from the test file location
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

-- Source the plugin manually (plenary busted runs with --noplugin)
vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe("KQL LSP smoke test", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should have the binary available", function()
    assert.equals(1, vim.fn.executable(binary),
      "KQL LSP binary should exist at: " .. binary)
  end)

  it("should attach LSP client to a .kql buffer", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    -- Wait for the LSP client to attach
    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "KQL LSP client should attach within 10 seconds")
    assert.equals("kql-lsp", client.name)
  end)

  it("should have initialized successfully", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "KQL LSP client should attach")

    -- Wait a moment for initialize to complete
    vim.wait(2000, function() return false end, 100)

    -- Check server info from initialization
    assert.is_not_nil(client.config, "Client should have config")
    assert.equals("kql-lsp", client.name)
  end)
end)
