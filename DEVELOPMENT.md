# KQL LSP — TDD Development Workflow

## Philosophy

Every feature follows the same loop:

1. **Write a failing Neovim test** (~2-3 seconds to run)
2. **Implement the feature** in the Rust LSP binary
3. **Neovim test passes** — feature works via stdio
4. **Write/verify IntelliJ UI test** (~20 seconds to run)
5. **IntelliJ test passes** — feature works in the IDE
6. **Done.** Commit.

Neovim tests are the fast backbone. IntelliJ tests are the gate. A feature is NOT done until both pass.

## Example: Adding Diagnostics

### Step 1: Neovim Test (Write First)

Create `neovim/test/diagnostics_spec.lua`:

```lua
local this_file = debug.getinfo(1, "S").source:sub(2):gsub("\\", "/")
local test_dir = vim.fn.fnamemodify(this_file, ":h")
local neovim_dir = vim.fn.fnamemodify(test_dir, ":h")
local project_root = vim.fn.fnamemodify(neovim_dir, ":h")
local helpers = dofile(test_dir .. "/helpers.lua")

local binary = project_root .. "/lsp/target/release/kql-lsp.exe"
vim.g.kql_lsp_binary = binary
dofile(neovim_dir .. "/plugin/kql.lua")

describe("KQL diagnostics", function()
  after_each(function() helpers.cleanup() end)

  it("should report error for invalid syntax", function()
    local bufnr = helpers.open_kql_buffer("StormEvents | where")

    local diagnostics = helpers.wait_for_diagnostics(bufnr, 10000)
    assert.is_true(#diagnostics > 0, "Should produce at least one diagnostic")
    assert.equals(vim.diagnostic.severity.ERROR, diagnostics[1].severity)
  end)
end)
```

Run it (should FAIL — LSP doesn't produce diagnostics yet):
```powershell
"C:/Program Files/Neovim/bin/nvim.exe" --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/diagnostics_spec.lua"
```

### Step 2: Implement in Rust

Edit `lsp/src/main.rs` to add diagnostic logic in `handle_did_open` and `handle_did_change`.

Build:
```powershell
cd lsp && cargo build --release
```

### Step 3: Neovim Test Passes

Re-run the same command. Should pass in ~3 seconds.

### Step 4: IntelliJ UI Test

Add to `LspIntegrationTest.kt`:
```kotlin
@Test
@Order(8)
fun `08 - verify diagnostics appear in editor`() {
    // ... (same DocumentMarkupModel pattern from educational-lsp-intellij)
}
```

Restart IDE sandbox (LSP binary changed):
```powershell
# Kill old IDE, then restart
cd intellij && ./gradlew runIdeForUiTests
```

Run UI tests:
```powershell
cd intellij && ./gradlew uiTest
```

### Step 5: Both Pass → Done

Commit the feature.

## Setup

### First Time

```powershell
# 1. Build LSP binary
cd lsp && cargo build --release

# 2. Verify Neovim tests
cd .. && "C:/Program Files/Neovim/bin/nvim.exe" --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/smoke_spec.lua"

# 3. Build IntelliJ plugin
cd intellij && ./gradlew build

# 4. Start IDE sandbox
cd intellij && ./gradlew runIdeForUiTests
# Wait for IDE to fully start (~30-60 seconds)

# 5. Run IntelliJ tests
cd intellij && ./gradlew uiTest
```

### Daily Development

```powershell
# Start IDE sandbox once per session
cd intellij && ./gradlew runIdeForUiTests &

# Fast loop (Neovim)
cd lsp && cargo build --release
cd .. && "C:/Program Files/Neovim/bin/nvim.exe" --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/YOUR_SPEC.lua"

# Gate (IntelliJ) — after Neovim tests pass
cd intellij && ./gradlew uiTest
```

## Troubleshooting

### IDE sandbox won't start
```powershell
powershell -ExecutionPolicy Bypass -File scripts/kill-lsp-zombies.ps1
cd intellij && ./gradlew --stop
cd intellij && ./gradlew runIdeForUiTests
```

### UI tests fail with "Unable to create converter for RetrieveResponse"
The `--add-opens` JVM args are missing. Check `build.gradle.kts` uiTest task.

### LSP binary won't start (lsp4ij crash loop)
1. Check `lsp/app.log` for errors
2. Make sure `flush()` is called after every write
3. Make sure you handle both string and numeric JSON-RPC IDs
4. Run `scripts/test-lsp-initialize.ps1` to test the binary directly

### Neovim test hangs
The LSP binary probably isn't flushing stdout. Add `writer.flush()` after every `write!()`.

### Stale Gradle test results
```powershell
cd intellij && ./gradlew --stop
rm -rf intellij/build/test-results/uiTest
```
