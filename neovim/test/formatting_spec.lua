-- Test document formatting (Slice 18)
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

describe('KQL formatting', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should put pipe operators on new lines', function()
        local text = 'StormEvents|where X>5|take 10'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/formatting', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            options = { tabSize = 4, insertSpaces = true },
        }, 5000)

        assert.is_not_nil(result, 'Should get formatting response')

        local edits = nil
        for _, res in pairs(result) do
            if res.result then
                edits = res.result
                break
            end
        end

        assert.is_not_nil(edits, 'Should have text edits')
        assert.is_true(#edits > 0, 'Should have at least one edit')

        -- Apply edits and check result
        vim.lsp.util.apply_text_edits(edits, bufnr, 'utf-16')
        local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
        local formatted = table.concat(lines, '\n')

        -- Should have pipes on new lines
        assert.is_true(formatted:find('\n|') ~= nil, 'Should have pipes on new lines, got: ' .. formatted)
    end)

    it('should add spaces around binary operators', function()
        local text = 'StormEvents | where X>5'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/formatting', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            options = { tabSize = 4, insertSpaces = true },
        }, 5000)

        assert.is_not_nil(result, 'Should get formatting response')

        local edits = nil
        for _, res in pairs(result) do
            if res.result then
                edits = res.result
                break
            end
        end

        assert.is_not_nil(edits, 'Should have text edits')

        -- Apply edits
        vim.lsp.util.apply_text_edits(edits, bufnr, 'utf-16')
        local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
        local formatted = table.concat(lines, '\n')

        -- Should have spaces around >
        assert.is_true(formatted:find('X > 5') ~= nil, 'Should have spaces around operators, got: ' .. formatted)
    end)

    it('should not change already well-formatted text', function()
        local text = 'StormEvents\n| where X > 5\n| take 10'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/formatting', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            options = { tabSize = 4, insertSpaces = true },
        }, 5000)

        assert.is_not_nil(result, 'Should get formatting response')

        local edits = nil
        for _, res in pairs(result) do
            if res.result then
                edits = res.result
                break
            end
        end

        -- Either no edits or empty edits array
        local is_empty = edits == nil or #edits == 0
        assert.is_true(is_empty, 'Well-formatted text should produce no edits')
    end)
end)
