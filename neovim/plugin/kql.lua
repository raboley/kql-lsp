-- KQL LSP Neovim plugin
-- Sets up LSP client for .kql files using vim.lsp.start

vim.api.nvim_create_autocmd("FileType", {
  pattern = "kql",
  callback = function(args)
    local binary = vim.g.kql_lsp_binary
    if not binary or vim.fn.executable(binary) == 0 then
      vim.notify("KQL LSP binary not found: " .. (binary or "nil"), vim.log.levels.WARN)
      return
    end

    vim.lsp.start({
      name = "kql-lsp",
      cmd = { binary },
      root_dir = vim.fn.getcwd(),
      init_options = vim.g.kql_lsp_init_options or {},
    })
  end,
})
