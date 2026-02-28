-- Minimal init.lua for headless Neovim LSP testing with plenary.nvim
-- Usage: nvim --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/smoke_spec.lua"

-- Determine paths
local home = vim.fn.expand("~")
local plenary_path = home .. "/git/plenary.nvim"

-- Auto-clone plenary.nvim if missing
if vim.fn.isdirectory(plenary_path) == 0 then
  print("Cloning plenary.nvim...")
  vim.fn.system({
    "git", "clone", "--depth", "1",
    "https://github.com/nvim-lua/plenary.nvim.git",
    plenary_path,
  })
  if vim.v.shell_error ~= 0 then
    error("Failed to clone plenary.nvim to " .. plenary_path)
  end
  print("plenary.nvim cloned successfully")
end

-- Add plenary to runtime path and load its plugin files
-- (--noplugin skips plugin/ dirs, so we must source them manually)
vim.opt.runtimepath:append(plenary_path)
vim.cmd("runtime plugin/plenary.vim")

-- Get the project root (parent of neovim/)
local script_dir = debug.getinfo(1, "S").source:sub(2)
local project_root = vim.fn.fnamemodify(script_dir, ":h:h")
-- Normalize path separators for Windows
project_root = project_root:gsub("\\", "/")

-- Add the neovim/ dir as a plugin so plugin/kql.lua gets loaded
vim.opt.runtimepath:append(project_root .. "/neovim")

-- Store project root for tests to find the binary
vim.g.kql_lsp_root = project_root

-- Set the binary path (tests and plugin use this)
local binary_path = project_root .. "/lsp/target/release/kql-lsp.exe"
if vim.fn.has("unix") == 1 then
  binary_path = project_root .. "/lsp/target/release/kql-lsp"
end
vim.g.kql_lsp_binary = binary_path

-- Add neovim/test to Lua package.path so tests can require("test.helpers")
package.path = project_root .. "/neovim/?.lua;" .. package.path

-- Register .kql filetype
vim.filetype.add({
  extension = {
    kql = "kql",
  },
})
