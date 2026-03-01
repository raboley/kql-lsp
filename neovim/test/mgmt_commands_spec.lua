-- Test management commands parsing (Slice 14)
local this_file = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/")
local test_dir = vim.fn.fnamemodify(this_file, ":h")
local neovim_dir = vim.fn.fnamemodify(test_dir, ":h")
local project_root = vim.fn.fnamemodify(neovim_dir, ":h")

local helpers = dofile(test_dir .. "/helpers.lua")

local binary = project_root .. "/lsp/target/release/kql-lsp.exe"
if vim.fn.has("unix") == 1 then
  binary = project_root .. "/lsp/target/release/kql-lsp"
end

vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe('KQL management commands', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should parse .show tables without diagnostics', function()
        local text = '.show tables'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Wait for diagnostics to settle
        vim.wait(3000, function() return false end, 100)

        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Should have zero diagnostics for .show tables')
    end)

    it('should parse .create table command without diagnostics', function()
        local text = '.create table MyTable (Name: string, Age: int)'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)

        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Should have zero diagnostics for .create table')
    end)

    it('should show management command in document symbols', function()
        local text = '.show tables'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/documentSymbol', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
        }, 5000)

        assert.is_not_nil(result, 'Should get symbol response')

        local symbols = nil
        for _, res in pairs(result) do
            if res.result then
                symbols = res.result
                break
            end
        end

        assert.is_not_nil(symbols, 'Should have symbols')
        assert.is_true(#symbols > 0, 'Should have at least one symbol for management command')
    end)
end)
