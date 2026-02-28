-- Test code actions for KQL (Slice 17)
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

describe('KQL code actions', function()
    after_each(function()
        helpers.cleanup()
    end)

    it('should offer "Add missing semicolon" for let without semicolon', function()
        local text = 'let x = 10\nStormEvents | take 5'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        -- Wait for diagnostics
        vim.wait(3000, function() return false end, 100)

        -- Request code actions at line 0 (the let statement)
        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/codeAction', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            range = {
                start = { line = 0, character = 0 },
                ['end'] = { line = 0, character = 10 },
            },
            context = { diagnostics = {} },
        }, 5000)

        assert.is_not_nil(result, 'Should get code action response')

        local actions = nil
        for _, res in pairs(result) do
            if res.result then
                actions = res.result
                break
            end
        end

        assert.is_not_nil(actions, 'Should have code actions')
        assert.is_true(#actions > 0, 'Should have at least one code action')

        -- Check that one action is "Add missing semicolon"
        local has_semicolon_action = false
        for _, action in ipairs(actions) do
            if action.title and action.title:find('semicolon') then
                has_semicolon_action = true
            end
        end
        assert.is_true(has_semicolon_action, 'Should have semicolon fix action')
    end)

    it('should not offer semicolon fix when let already has one', function()
        local text = 'let x = 10;\nStormEvents | take 5'
        local bufnr = helpers.open_kql_buffer(text)
        local client = helpers.wait_for_client(bufnr, 10000)
        assert.is_not_nil(client, 'LSP client should attach')

        vim.wait(3000, function() return false end, 100)

        local result = vim.lsp.buf_request_sync(bufnr, 'textDocument/codeAction', {
            textDocument = vim.lsp.util.make_text_document_params(bufnr),
            range = {
                start = { line = 0, character = 0 },
                ['end'] = { line = 0, character = 11 },
            },
            context = { diagnostics = {} },
        }, 5000)

        assert.is_not_nil(result, 'Should get code action response')

        local actions = nil
        for _, res in pairs(result) do
            if res.result then
                actions = res.result
                break
            end
        end

        -- Should have no actions, or empty list
        local has_semicolon_action = false
        if actions then
            for _, action in ipairs(actions) do
                if action.title and action.title:find('semicolon') then
                    has_semicolon_action = true
                end
            end
        end
        assert.is_false(has_semicolon_action, 'Should NOT have semicolon fix when semicolon exists')
    end)
end)
