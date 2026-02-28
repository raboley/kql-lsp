-- Test find references for let-bound variables (Slice 12)
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

describe('KQL find references', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should find all references to a let-bound variable', function()
        local text = 'let x = 10;\nlet y = x + 1;\nStormEvents | where Col > x'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Request references for "x" on line 2, character 26 (last "x")
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/references', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 2, character = 26 },
            context = { includeDeclaration = true },
        }, 5000)

        assert.is_not_nil(result, 'Should get references response')

        local locations = nil
        for _, res in pairs(result) do
            if res.result then
                locations = res.result
                break
            end
        end

        assert.is_not_nil(locations, 'Should find references')
        assert.are.equal(3, #locations, 'Should find 3 references (declaration + 2 usages), got ' .. #locations)
    end)

    it('should return empty for identifiers with no references', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | take 10')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Request references for "StormEvents" (not a let-bound variable)
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/references', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 3 },
            context = { includeDeclaration = true },
        }, 5000)

        assert.is_not_nil(result, 'Should get references response')

        local locations = nil
        for _, res in pairs(result) do
            if res.result then
                locations = res.result
                break
            end
        end

        local is_empty = locations == nil
            or (type(locations) == 'table' and #locations == 0)
            or locations == vim.NIL
        assert.is_true(is_empty, 'Should return empty for non-let identifiers')
    end)
end)
