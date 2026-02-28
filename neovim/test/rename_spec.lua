-- Test rename symbol for let-bound variables (Slice 20)
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

describe('KQL rename symbol', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should rename a let-bound variable across all usages', function()
        local text = 'let threshold = 100;\nStormEvents | where Damage > threshold'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Request rename for "threshold" on line 1
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/rename', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 1, character = 35 },
            newName = 'minDamage',
        }, 5000)

        assert.is_not_nil(result, 'Should get rename response')

        local edit = nil
        for _, res in pairs(result) do
            if res.result then
                edit = res.result
                break
            end
        end

        assert.is_not_nil(edit, 'Should get workspace edit')
        assert.is_not_nil(edit.changes, 'Should have changes')

        -- Find the edits for our document
        local doc_edits = nil
        for _, edits in pairs(edit.changes) do
            doc_edits = edits
            break
        end

        assert.is_not_nil(doc_edits, 'Should have document edits')
        assert.are.equal(2, #doc_edits, 'Should have 2 edits (declaration + usage)')
    end)

    it('should return null for non-let identifiers', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | take 10')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/rename', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 3 },
            newName = 'NewName',
        }, 5000)

        assert.is_not_nil(result, 'Should get response')

        local edit = nil
        for _, res in pairs(result) do
            if res.result then
                edit = res.result
                break
            end
        end

        -- Should be null or have empty changes
        local is_empty = edit == nil
            or edit == vim.NIL
            or (edit.changes and next(edit.changes) == nil)
        assert.is_true(is_empty, 'Should return empty for non-let identifiers')
    end)
end)
