-- Schema diagnostics tests: verify unknown tables/columns produce warnings
-- Slice 24: Schema-Aware Diagnostics

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

-- Set init_options with schemaFile BEFORE sourcing the plugin
local schema_path = project_root .. "/neovim/test-data/.kql-schema.json"
vim.g.kql_lsp_init_options = { schemaFile = schema_path }
vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe("KQL schema diagnostics", function()
  after_each(function()
    helpers.cleanup()
  end)

  it("should warn on unknown table name", function()
    local bufnr = helpers.open_kql_buffer("NonExistentTable | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait for diagnostics to arrive
    local diagnostics = helpers.wait_for_diagnostics(bufnr, 10000)

    -- Find a diagnostic about the unknown table
    local found_table_warning = false
    for _, d in ipairs(diagnostics) do
      if d.message:match("NonExistentTable") then
        found_table_warning = true
        -- Static schema should produce WARNING (severity 2)
        assert.are.equal(vim.diagnostic.severity.WARN, d.severity,
          "Static schema should produce WARNING, not ERROR")
      end
    end

    assert.is_true(found_table_warning, "Should have a diagnostic about 'NonExistentTable'")
  end)

  it("should not warn on known table name", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | take 10")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    -- Wait briefly, then check diagnostics
    vim.wait(3000, function() return false end, 100)
    local diagnostics = vim.diagnostic.get(bufnr)

    -- Filter out parse-error diagnostics (we only care about schema diagnostics)
    local schema_diags = {}
    for _, d in ipairs(diagnostics) do
      if d.message:match("unknown") or d.message:match("Unknown") then
        table.insert(schema_diags, d)
      end
    end

    assert.are.equal(0, #schema_diags, "Known table should not produce schema diagnostics")
  end)

  it("should warn on unknown column name", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | where FakeColumn > 5")

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    local diagnostics = helpers.wait_for_diagnostics(bufnr, 10000)

    local found_column_warning = false
    for _, d in ipairs(diagnostics) do
      if d.message:match("FakeColumn") then
        found_column_warning = true
        assert.are.equal(vim.diagnostic.severity.WARN, d.severity,
          "Static schema should produce WARNING for unknown column")
      end
    end

    assert.is_true(found_column_warning, "Should have a diagnostic about 'FakeColumn'")
  end)

  it("should not warn on known column name", function()
    local bufnr = helpers.open_kql_buffer('StormEvents | where State == "TX"')

    local client = helpers.wait_for_client(bufnr, 10000)
    assert.is_not_nil(client, "LSP client should attach")

    vim.wait(3000, function() return false end, 100)
    local diagnostics = vim.diagnostic.get(bufnr)

    local schema_diags = {}
    for _, d in ipairs(diagnostics) do
      if d.message:match("unknown") or d.message:match("Unknown") then
        table.insert(schema_diags, d)
      end
    end

    assert.are.equal(0, #schema_diags, "Known column should not produce schema diagnostics")
  end)
end)
