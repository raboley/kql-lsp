-- Test signature help for built-in functions (Slice 16)
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

describe('KQL signature help', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should show signature for ago() inside parens', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where StartTime > ago(')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/signatureHelp', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 37 },
        }, 5000)

        assert.is_not_nil(result, 'Should get signature help response')

        local sig = nil
        for _, res in pairs(result) do
            if res.result then
                sig = res.result
                break
            end
        end

        assert.is_not_nil(sig, 'Should have signature help result')
        assert.is_not_nil(sig.signatures, 'Should have signatures')
        assert.is_true(#sig.signatures > 0, 'Should have at least one signature')
        assert.is_truthy(sig.signatures[1].label:find('ago'), 'Signature should mention ago')
    end)

    it('should show active parameter for strcat with comma', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | extend X = strcat("a", ')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/signatureHelp', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 38 },
        }, 5000)

        assert.is_not_nil(result, 'Should get signature help response')

        local sig = nil
        for _, res in pairs(result) do
            if res.result then
                sig = res.result
                break
            end
        end

        assert.is_not_nil(sig, 'Should have signature help')
        assert.is_not_nil(sig.signatures, 'Should have signatures')
        assert.is_true(#sig.signatures > 0, 'Should have signature for strcat')
        -- Active parameter should be 1 (second parameter, after the comma)
        assert.are.equal(1, sig.activeParameter, 'Active parameter should be 1 (after comma)')
    end)

    it('should return null outside function parens', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | take 10')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/signatureHelp', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 5 },
        }, 5000)

        assert.is_not_nil(result, 'Should get response')

        local sig = nil
        for _, res in pairs(result) do
            if res.result then
                sig = res.result
                break
            end
        end

        -- Should be null or have empty signatures
        local is_empty = sig == nil
            or sig == vim.NIL
            or (sig.signatures and #sig.signatures == 0)
        assert.is_true(is_empty, 'Should return empty outside function call')
    end)
end)
