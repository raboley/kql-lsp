# Production-Grade KQL LSP Development Plan

## Context

Build a full-featured LSP for Microsoft's Kusto Query Language (KQL) in Rust, with plugins for Neovim and IntelliJ. The monorepo has three parts: `lsp/` (Rust binary), `neovim/` (plugin + tests), `intellij/` (plugin + UI tests).

## Core Principle: Vertical Feature Slices

Every slice delivers a thin, end-to-end feature: write a Neovim test, implement in Rust, verify in Neovim, write an IntelliJ test, verify in IntelliJ, commit. **No slice is done until both editors prove the feature works.** See `DEVELOPMENT.md` for the full TDD workflow.

## Architecture Decisions

| Area | Choice | Rationale |
|------|--------|-----------|
| **Rope** | `ropey 1.6` | Most mature Rust rope; built-in UTF-16 conversion for LSP protocol |
| **LSP transport** | Keep `rpc.rs` + add `lsp-types 0.97` | Current transport works; `lsp-types` adds typed structs on top |
| **Parser** | Hand-written recursive descent | Error recovery at `|` sync points; proven by rust-analyzer |
| **Syntax trees** | `rowan 0.15` (lossless CST) | Preserves all text; enables formatting, refactoring, precise diagnostics |
| **No async runtime** | Synchronous stdio loop | Simple, sufficient, debuggable |

## Feature Plans

Detailed slice definitions live in `plan/`:

| File | Status | Slices | Description |
|------|--------|--------|-------------|
| [plan/completed-slices-01-20.md](plan/completed-slices-01-20.md) | Done | 1–20 | Core LSP: parser, diagnostics, completion, hover, go-to-def, references, rename, formatting, folding, semantic tokens, etc. |
| [plan/adx-schema-completion.md](plan/adx-schema-completion.md) | Next | 21–26 | ADX schema integration: live cluster fetch, table/column completion, schema-aware diagnostics, schema hover |

## Target Module Structure

```
lsp/src/
  main.rs, rpc.rs, server.rs, document.rs, syntax.rs
  diagnostics.rs, semantic_tokens.rs, symbols.rs, hover.rs
  definition.rs, references.rs, code_actions.rs, formatting.rs
  signature_help.rs, folding.rs, rename.rs
  schema.rs, config.rs, adx.rs                              # NEW (slices 21-26)
  lexer/ { mod.rs, token.rs, lexer.rs }
  parser/ { mod.rs, parser.rs, grammar/ { statements.rs, operators.rs, expressions.rs, commands.rs } }
```

## Verification Strategy

Every slice follows the TDD workflow from `DEVELOPMENT.md`:

1. **Write failing Neovim test** (~2-3 sec)
2. **Implement in Rust** (cargo build + cargo test)
3. **Neovim test passes**
4. **Write IntelliJ UI test** (~20 sec)
5. **IntelliJ test passes**
6. **Commit**

```bash
cd lsp && cargo build --release          # Build
cd lsp && cargo test                     # Unit tests
nvim --headless ... -c "PlenaryBustedFile neovim/test/<spec>.lua"  # Neovim
cd intellij && ./gradlew uiTest         # IntelliJ
```

## Critical Files

- `lsp/src/main.rs` -- Message dispatch, extended each slice
- `lsp/src/rpc.rs` -- Transport layer (stable)
- `lsp/Cargo.toml` -- Dependencies
- `neovim/test/helpers.lua` -- Shared test helpers
- `intellij/src/test/kotlin/com/kqllsp/ui/LspIntegrationTest.kt` -- UI tests

## Risks

| Risk | Mitigation |
|------|-----------|
| No formal KQL grammar | Start small, grow incrementally; error recovery handles unknown syntax |
| UTF-16 offset bugs | Unit test in Slice 1 before anything depends on it |
| Management commands are vast | Parse loosely first, add typed parsing per-command as separate slices |
| Built-in function catalog (200+) | Start with ~15, expand incrementally |
| IntelliJ UI test flakiness | Generous timeouts; IntelliJ is the gate, Neovim is the fast loop |
| ADX connectivity | Graceful fallback when offline; static schema file as alternative |
