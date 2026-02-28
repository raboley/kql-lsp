# Instructions for Claude

## What This Repo Is

A KQL (Kusto Query Language) LSP with plugins for both Neovim and IntelliJ. The monorepo has three parts:
- `lsp/` - The Rust LSP binary
- `neovim/` - Neovim plugin + tests
- `intellij/` - IntelliJ plugin + Remote-Robot UI tests

## Mandatory TDD Development Loop

**A feature is NOT done until it passes in both Neovim AND IntelliJ.**

### Fast Loop (Neovim — use this most of the time)
1. Write a failing test in `neovim/test/`
2. Implement the feature in `lsp/src/`
3. Build: `cd lsp && cargo build --release`
4. Run: `"C:/Program Files/Neovim/bin/nvim.exe" --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/YOUR_SPEC.lua"`
5. Iterate until green (~2-3 seconds per run)

### Gate (IntelliJ — run after Neovim tests pass)
1. Write/update UI test in `intellij/src/test/kotlin/com/kqllsp/ui/`
2. Build plugin: `cd intellij && ./gradlew build`
3. Start IDE sandbox: `cd intellij && ./gradlew runIdeForUiTests` (if not running)
4. Run tests: `cd intellij && ./gradlew uiTest`
5. Must pass before feature is considered done (~20 seconds)

### When to Restart the IDE Sandbox
- **LSP binary changes**: YES — restart IDE (lsp4ij caches the process)
- **Plugin code changes** (Java/plugin.xml): YES — restart IDE
- **Test-only changes** (LspIntegrationTest.kt): NO — just re-run `./gradlew uiTest`

## Critical Rules

### LSP Binary (Rust)
- **ALWAYS `writer.flush()` after writing to stdout.** Without flush, pipe-based communication (lsp4ij, Neovim) buffers indefinitely.
- **ALWAYS handle both string and numeric JSON-RPC IDs.** lsp4ij sends `"id": "1"` (string), Neovim sends `"id": 1` (number).
- **NEVER use `writeln!` for LSP messages.** The extra `\n` corrupts the Content-Length framing.
- **Log to `app.log` in append mode.** Use `env_logger` with `Target::Pipe(Box::new(log_file))`.

### IntelliJ UI Tests (Kotlin)
- **Use `callJs<String>` with `"" + boolExpr`, never `callJs<Boolean>`.** Boolean is not Serializable in remote-robot protocol.
- **Never open projects from tests.** Let IDE restore its last project on startup. Use `invokeLater` for file ops.
- **Use `var` not `const`/`let` in callJs scripts** (Rhino JS engine = ES5).
- **JDK 17+ requires `--add-opens`** in uiTest task for GSON reflection.
- **IDE sandbox needs a project arg** in build.gradle.kts `runIdeForUiTests` to avoid exiting immediately on first launch.
- **Start IDE with PowerShell `Start-Process`**, not bash `&` — bash may not pass the display context correctly.

### Neovim Tests (Lua)
- **Use `dofile()` to load helpers and plugin**, not `require()` — plenary busted resets package.path.
- **Set `vim.g.kql_lsp_binary` before loading the plugin** so the autocmd can find the binary.
- **Use `vim.wait(timeout, condition, interval)` for async operations.**
- **Always `helpers.cleanup()` in `after_each`** to stop LSP clients between tests.

## Quick Commands

```powershell
# Build LSP
cd lsp && cargo build --release

# Run LSP unit tests
cd lsp && cargo test

# Run Neovim tests (~2-3 seconds)
cd kql-lsp && "C:/Program Files/Neovim/bin/nvim.exe" --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/smoke_spec.lua"

# Build IntelliJ plugin
cd intellij && ./gradlew build

# Start IDE sandbox (background)
cd intellij && ./gradlew runIdeForUiTests

# Run IntelliJ UI tests (~20 seconds)
cd intellij && ./gradlew uiTest

# Run everything
powershell -ExecutionPolicy Bypass -File scripts/test-all.ps1

# Kill zombie LSP processes
powershell -ExecutionPolicy Bypass -File scripts/kill-lsp-zombies.ps1
```

## File Structure

```
kql-lsp/
  CLAUDE.md                          # This file
  DEVELOPMENT.md                     # TDD workflow documentation
  scripts/
    test-all.ps1                     # Build + test everything
    test-neovim.ps1                  # Build + Neovim tests only
    test-intellij.ps1                # Build + IntelliJ tests only
    test-lsp-initialize.ps1          # Standalone LSP binary smoke test
    kill-lsp-zombies.ps1             # Emergency cleanup
  lsp/
    Cargo.toml
    src/main.rs                      # LSP stdin/stdout loop + message handlers
    src/rpc.rs                       # Content-Length framing, flush(), encode/decode
  neovim/
    minimal_init.lua                 # Headless test bootstrap (auto-clones plenary)
    plugin/kql.lua                   # Neovim plugin: filetype + vim.lsp.start
    test/helpers.lua                 # Shared: open_kql_buffer, wait_for_client, cleanup
    test/smoke_spec.lua              # Smoke: LSP starts and responds to initialize
    test-data/test.kql               # Sample KQL file
  intellij/
    build.gradle.kts                 # Gradle with lsp4ij + remote-robot deps
    gradle.properties                # Plugin metadata
    settings.gradle.kts
    src/main/java/com/kqllsp/
      KqlLspServer.java              # Launches the LSP binary
      KqlLspServerFactory.java       # Factory boilerplate
    src/main/resources/META-INF/
      plugin.xml                     # *.kql -> kqlLspServer
    src/test/kotlin/com/kqllsp/ui/
      LspIntegrationTest.kt          # Remote-Robot UI tests
    test-project/test.kql            # Triggers LSP in IDE sandbox
```
