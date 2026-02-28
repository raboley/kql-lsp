-- Test join operator parsing (Slice 13)
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

describe('KQL join operator', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should parse basic join without errors', function()
        local text = 'T1 | join (T2) on Key'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Basic join should parse without errors, got: ' .. vim.inspect(diagnostics))
    end)

    it('should parse join with kind without errors', function()
        local text = 'T1 | join kind=inner (T2) on Key'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'Join with kind should parse without errors')
    end)

    it('should highlight join as keyword in semantic tokens', function()
        local bufnr = helpers.open_kql_buffer('T1 | join (T2) on Key')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/semanticTokens/full', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
        }, 5000)

        assert.is_not_nil(result, 'Should get semantic tokens response')
        local data = nil
        for _, res in pairs(result) do
            if res.result and res.result.data then
                data = res.result.data
                break
            end
        end
        assert.is_not_nil(data, 'Should have token data')
        -- Should have tokens for: T1, |, join, (, T2, ), on(identifier), Key
        local token_count = #data / 5
        assert.is_true(token_count >= 5, 'Should have at least 5 semantic tokens, got ' .. token_count)
    end)
end)
