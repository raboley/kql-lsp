-- Test go-to-definition for let-bound variables (Slice 11)
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

describe('KQL go-to-definition', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should jump to let declaration from variable usage', function()
        local bufnr = helpers.open_kql_buffer('let threshold = 100;\nStormEvents | where Damage > threshold')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Request definition for "threshold" on line 1, col 35 (inside "threshold" at end)
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/definition', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 1, character = 35 },
        }, 5000)

        assert.is_not_nil(result, 'Should get definition response')

        local locations = nil
        for _, res in pairs(result) do
            if res.result then
                locations = res.result
                break
            end
        end

        assert.is_not_nil(locations, 'Should find definition for threshold')

        -- Should point to line 0 (the let statement)
        local loc = locations
        if type(locations) == 'table' and locations[1] then
            loc = locations[1]
        end
        -- Location could be in locationLink or location format
        local target_line = loc.targetRange and loc.targetRange.start.line
            or (loc.range and loc.range.start.line)
        assert.are.equal(0, target_line, 'Definition should be on line 0 (let statement)')
    end)

    it('should return null for undefined identifiers', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where Damage > 100')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Request definition for "Damage" (not a let-bound variable)
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/definition', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 22 },
        }, 5000)

        assert.is_not_nil(result, 'Should get definition response')

        local locations = nil
        for _, res in pairs(result) do
            if res.result then
                locations = res.result
                break
            end
        end

        -- Should be null or empty for unknown identifiers
        local is_empty = locations == nil
            or (type(locations) == 'table' and #locations == 0)
            or locations == vim.NIL
        assert.is_true(is_empty, 'Should return empty for undefined identifier, got: ' .. vim.inspect(locations))
    end)
end)
