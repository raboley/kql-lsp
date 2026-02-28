-- Test multiple statements and statement separation (Slice 15)
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

describe('KQL multiple statements', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should parse two valid queries separated by blank line with zero diagnostics', function()
        local text = 'StormEvents | take 10\n\nOtherTable | where State == "TEXAS"'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Wait for diagnostics to be published (give it time)
        vim.wait(3000, function() return false end, 100)

        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Two valid queries should produce zero diagnostics')
    end)

    it('should show diagnostic only on invalid query, not valid one', function()
        local text = 'StormEvents | take 10\n\nOtherTable | where'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local diagnostics = helpers.wait_for_diagnostics(bufnr, 5000)
        assert.is_true(#diagnostics > 0, 'Should have diagnostic for incomplete where')

        -- Diagnostic should NOT be on line 0 (the valid query)
        for _, d in ipairs(diagnostics) do
            assert.is_true(d.lnum >= 2, 'Diagnostic should be on the invalid query (line >= 2), got lnum=' .. d.lnum)
        end
    end)

    it('should parse let chains with semicolons before a query', function()
        local text = 'let x = 1;\nlet y = 2;\nStormEvents | where Col > x'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Wait for diagnostics to settle
        vim.wait(3000, function() return false end, 100)

        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Let chains with query should produce zero diagnostics')
    end)

    it('should produce correct document symbols for multiple statements', function()
        local text = 'let threshold = 100;\n\nStormEvents | take 10\n\nT2 | where X > 5'
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
        assert.are.equal(3, #symbols, 'Should have 3 symbols: threshold, StormEvents, T2')
    end)
end)
