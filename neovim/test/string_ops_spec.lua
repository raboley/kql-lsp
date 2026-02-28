-- Test string operators, logical operators, and timespan literals (Slice 10)
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

describe('KQL string operators and timespan literals', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should parse where with contains operator without errors', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where State contains "TEX"')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Wait for LSP to process
        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'contains should parse without errors, got: ' .. vim.inspect(diagnostics))
    end)

    it('should parse where with has operator without errors', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where Name has "storm"')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'has should parse without errors')
    end)

    it('should parse where with and/or logical operators', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where State contains "TEX" and DamageProperty > 0')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'and should parse without errors')
    end)

    it('should parse ago() with timespan literal', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where StartTime > ago(1h)')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)
        local diagnostics = vim.diagnostic.get(bufnr)
        assert.are.equal(0, #diagnostics, 'ago(1h) should parse without errors')
    end)

    it('should highlight contains as operator in semantic tokens', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where State contains "TEX"')
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
        -- StormEvents(property), |(operator), where(keyword), State(property), contains(operator), "TEX"(string) = 6 tokens
        local token_count = #data / 5
        assert.is_true(token_count >= 6, 'Should have at least 6 tokens, got ' .. token_count)
    end)

    it('should provide hover for contains operator', function()
        local bufnr = helpers.open_kql_buffer('StormEvents | where State contains "TEX"')
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Position on "contains" (col 33 is inside "contains")
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/hover', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            position = { line = 0, character = 33 },
        }, 5000)

        assert.is_not_nil(result, 'Should get hover response')

        local hover = nil
        for _, res in pairs(result) do
            if res.result then
                hover = res.result
                break
            end
        end

        assert.is_not_nil(hover, 'Should get hover for contains')
        assert.is_not_nil(hover.contents, 'Should have contents')
        assert.is_truthy(hover.contents.value:find('contains'), 'Hover should mention contains')
    end)
end)
