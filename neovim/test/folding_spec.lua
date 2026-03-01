-- Test folding ranges (Slice 19)
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

describe('KQL folding ranges', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should return folding range for multi-line query', function()
        local text = 'StormEvents\n| where State == "TEXAS"\n| take 10'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/foldingRange', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
        }, 5000)

        assert.is_not_nil(result, 'Should get folding range response')

        local ranges = nil
        for _, res in pairs(result) do
            if res.result then
                ranges = res.result
                break
            end
        end

        assert.is_not_nil(ranges, 'Should have folding ranges')
        assert.is_true(#ranges > 0, 'Should have at least one folding range')

        -- The range should span from line 0 to line 2
        local found_range = false
        for _, r in ipairs(ranges) do
            if r.startLine == 0 and r.endLine >= 2 then
                found_range = true
            end
        end
        assert.is_true(found_range, 'Should have a range spanning the multi-line query')
    end)

    it('should not return folding range for single-line query', function()
        local text = 'StormEvents | take 10'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/foldingRange', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
        }, 5000)

        assert.is_not_nil(result, 'Should get folding range response')

        local ranges = nil
        for _, res in pairs(result) do
            if res.result then
                ranges = res.result
                break
            end
        end

        -- Single-line queries have no foldable range
        local is_empty = ranges == nil or #ranges == 0
        assert.is_true(is_empty, 'Single-line query should have no folding ranges')
    end)
end)
