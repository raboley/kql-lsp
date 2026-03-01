# ADX Schema-Aware Completion & Diagnostics

## Context

The KQL LSP currently has keyword/operator completion after `|` and parser-error-only diagnostics. There is no awareness of table names, column names, or database schema. This feature adds ADX (Azure Data Explorer) schema integration so the LSP can:
- Connect to a live ADX cluster and fetch real schema via Azure CLI auth
- Complete table names at the start of queries
- Complete column names context-aware (after `where`, `project`, `extend`, etc.)
- Flag unknown tables/columns as diagnostics (Errors for live schema, Warnings for static)
- Export fetched schema to a static `.kql-schema.json` file for offline/testing use

**Target cluster for development:** `https://help.kusto.windows.net` / `Samples` database (the public ADX playground with real StormEvents data).

---

## Slice 21: Config + ADX Live Schema Fetching + Schema Types

**What ships:** The LSP reads configuration (from `initializationOptions` or `.kql-lsp.json`), connects to an ADX cluster at startup via `az` CLI auth, fetches the real database schema, and stores it in memory. Also supports loading from a static `.kql-schema.json` file. No completion/diagnostics changes yet — this slice is pure infrastructure.

**New files:**
- `lsp/src/schema.rs` — `DatabaseSchema`, `Table`, `Column`, `SchemaStore`, `SchemaSource` types. JSON (de)serialization. Lookup helpers: `table_names()`, `columns_for_table(name)`, `has_table(name)`, `has_column(table, column)`. `load_from_file(path)` for static schema.
- `lsp/src/config.rs` — `LspConfig` with optional `schema_file: String` and `adx: Option<AdxConfig>`. `AdxConfig { cluster: String, database: String }`. Parsing from `serde_json::Value` (initializationOptions) and from `.kql-lsp.json` file.
- `lsp/src/adx.rs` — `get_access_token()` (shells out to `az account get-access-token --resource https://api.kusto.windows.net`), `fetch_schema(cluster, database, token)` (HTTP POST to `{cluster}/v1/rest/mgmt` with body `{"db": db, "csl": ".show database {db} schema as json"}`), `parse_adx_response(json)` → `DatabaseSchema`.

**New Cargo.toml dependency:**
- `ureq = "2"` — synchronous HTTP client (no async runtime needed, uses rustls for TLS).

**Modified files:**
- `lsp/src/server.rs` — Add `schema: SchemaStore`, `config: LspConfig`, `schema_receiver: Option<mpsc::Receiver<Result<DatabaseSchema, String>>>` to `ServerState`.
- `lsp/src/main.rs` — In `handle_initialize`: extract `initializationOptions` and `rootUri`. Parse config (initializationOptions > `.kql-lsp.json` in workspace root). If `adx` config present, spawn `std::thread` to fetch schema via `adx.rs`, store receiver in state. If `schema_file` present (and no adx), load static schema synchronously. In main message loop: add `try_recv()` check — when live schema arrives, store with `SchemaSource::Live` and log success. Graceful fallback: if fetch fails, log warning, continue with no schema.
- `neovim/plugin/kql.lua` — Read `vim.g.kql_lsp_init_options` and pass as `init_options` to `vim.lsp.start()`.

**Test data — generated from real ADX:**
- After implementing, run the LSP against `https://help.kusto.windows.net` / `Samples` to fetch the real schema. Save the result as `neovim/test-data/.kql-schema.json` and `intellij/test-project/.kql-schema.json`. This becomes our static test fixture for all subsequent slices.

**Rust unit tests:**
- `parse_adx_response` with a fixture JSON captured from the real Help cluster response.
- `get_access_token` error handling when `az` is not installed.
- `LspConfig` parsing from JSON.
- `SchemaStore` lookup helpers.
- `load_from_file` round-trip.

**Neovim test (`neovim/test/schema_load_spec.lua`):**
- Set `vim.g.kql_lsp_init_options = { schemaFile = "<path>/.kql-schema.json" }` before loading plugin.
- Open a `.kql` buffer, wait for client.
- Verify LSP starts without errors (schema loaded silently in background).
- Config with invalid/unreachable ADX cluster → LSP starts normally, no crash.

**IntelliJ test:**
- Place `.kql-schema.json` + `.kql-lsp.json` in `test-project/`.
- Verify LSP processes files without errors.

---

## Slice 22: Table Name Completion

**What ships:** Typing at the start of a query offers table name completions from the loaded schema (live or static).

**Modified files:**
- `lsp/src/completion.rs` — Change signature: `complete_at(text, offset, schema: Option<&DatabaseSchema>)`. Add `CompletionContext::TableName` detection — cursor is at the start of a line or statement, not after a pipe. When detected, return table names from schema with `CompletionItemKind::Class` (7). Existing pipe completion unchanged.
- `lsp/src/main.rs` — Pass `&state.schema.schema` to `complete_at` in `handle_completion`.

**Neovim test (`neovim/test/table_completion_spec.lua`):**
- Uses static schema fixture generated in Slice 21 (real StormEvents tables).
- Buffer with `S` → completion at (0,1) → `StormEvents` appears.
- Buffer empty → completion at (0,0) → table names appear.
- Buffer with `StormEvents | ` → completion still returns tabular operators (existing behavior preserved).

**IntelliJ test:** Add example file `docs/examples/slice-22-table-completion.kql`, verify in log.

---

## Slice 23: Column Name Completion (Context-Aware)

**What ships:** After `StormEvents | where `, column names for StormEvents appear. Columns are context-aware — different tables show different columns.

**Modified files:**
- `lsp/src/completion.rs` — Add `CompletionContext::ColumnName { table_name }`. Implement `find_table_for_query(text, offset)` — walks backward through tokens to find the first identifier (table name) before any pipe in the current query. Implement `is_in_column_position(text, offset)` — detects cursor after `where`, `project`, `extend`, `summarize ... by`, `sort by`, `top ... by`, `distinct`, or after comma in column lists. Return columns with `CompletionItemKind::Field` (5) and column type as detail.

**Neovim test (`neovim/test/column_completion_spec.lua`):**
- Uses real schema fixture.
- `StormEvents | where ` → completions include real column names.
- `StormEvents | project ` → same columns.
- `StormEvents | where State == "TX" | project ` → still StormEvents columns.
- `UnknownTable | where ` → no column completions.

**IntelliJ test:** Verify column completion requests in LSP log.

---

## Slice 24: Schema-Aware Diagnostics

**What ships:** Unknown table names and column references produce diagnostics. Severity = ERROR for live schema, WARNING for static schema.

**Modified files:**
- `lsp/src/diagnostics.rs` — Add `schema_diagnostics(text, schema, rope)`. Walks query text to identify: (1) first identifier in each query statement → check `schema.has_table()`, (2) identifiers in column positions → check `schema.has_column(table, name)`. Skips built-in function names (via `catalog`), let-bound variable names, and keywords. Severity from `SchemaSource`.
- `lsp/src/main.rs` — Update `publish_diagnostics` to accept `&SchemaStore`, call `schema_diagnostics`, merge with parse error diagnostics. Update call sites in `handle_did_open` and `handle_did_change`.

**Neovim test (`neovim/test/schema_diagnostics_spec.lua`):**
- Uses real schema fixture (static → WARNING severity).
- `NonExistentTable | take 10` → WARNING on "NonExistentTable".
- `StormEvents | where FakeColumn > 5` → WARNING on "FakeColumn".
- `StormEvents | where State == "TX"` → zero schema diagnostics.
- `StormEvents | where count() > 5` → no warning on `count` (built-in function).
- Verify severity is WARNING (2), not ERROR (1), for static schema.

**IntelliJ test:** Verify warning diagnostics in LSP log for unknown table.

---

## Slice 25: Schema Hover (Tables and Columns)

**What ships:** Hovering over a table name shows its column list. Hovering over a column shows its type.

**Modified files:**
- `lsp/src/hover.rs` — Extend: if identifier is not a built-in function/operator, check schema. Table name → markdown with column list. Column name (in column position) → `column: type`.
- `lsp/src/main.rs` — Pass `&state.schema` to hover handler.

**Neovim test (`neovim/test/schema_hover_spec.lua`):**
- Hover over `StormEvents` → shows table with real column list.
- Hover over `State` in `StormEvents | where State == "TX"` → shows `State: string`.

**IntelliJ test:** Verify hover request processed in LSP log.

---

## Slice 26: Config File Watcher + Schema Reload

**What ships:** Schema reloads when config files change. Both editors have documented config.

**Modified files:**
- `lsp/src/main.rs` — Handle `workspace/didChangeWatchedFiles`. When `.kql-lsp.json` or `.kql-schema.json` changes, reload schema, re-publish diagnostics for all open documents. Register file watchers in capabilities.
- `neovim/plugin/kql.lua` — Document `vim.g.kql_lsp_init_options` usage in comments.

**Neovim test (`neovim/test/config_spec.lua`):**
- Verify init_options flow works.
- Verify `.kql-lsp.json` fallback in workspace root.

**IntelliJ test:** Verify config loading via `.kql-lsp.json` in project root.

---

## Key Files Summary

| File | Role |
|------|------|
| `lsp/src/schema.rs` | NEW — Schema types, JSON loading, lookup helpers |
| `lsp/src/config.rs` | NEW — LSP config parsing (init_options + file) |
| `lsp/src/adx.rs` | NEW — ADX REST client, Azure CLI token, schema fetch |
| `lsp/src/server.rs` | ADD schema + config + receiver to ServerState |
| `lsp/src/main.rs` | MODIFY initialize, message loop, publish_diagnostics, completion, hover |
| `lsp/src/completion.rs` | EXTEND with table/column context detection |
| `lsp/src/diagnostics.rs` | ADD schema_diagnostics function |
| `lsp/src/hover.rs` | EXTEND with schema-aware hover |
| `neovim/plugin/kql.lua` | ADD init_options pass-through |
| `lsp/Cargo.toml` | ADD `ureq = "2"` |

## Static Schema File Format (`.kql-schema.json`)

Generated from live ADX fetch, not hand-crafted:
```json
{
  "database": "Samples",
  "tables": [
    {
      "name": "StormEvents",
      "columns": [
        { "name": "StartTime", "type": "datetime" },
        { "name": "State", "type": "string" },
        ...
      ]
    },
    ...
  ]
}
```

## LSP Config File Format (`.kql-lsp.json`)

```json
{
  "schemaFile": ".kql-schema.json",
  "adx": {
    "cluster": "https://help.kusto.windows.net",
    "database": "Samples"
  }
}
```

Priority: `initializationOptions` > `.kql-lsp.json`. If both `schemaFile` and `adx` are set, `adx` takes priority (live overrides static).

## Development Sequence

1. Build Slice 21 (config + ADX fetch + schema types)
2. Use the working ADX fetch to connect to `help.kusto.windows.net/Samples` and save the real schema as `.kql-schema.json` test fixtures
3. Build Slices 22-26 using those real fixtures for all tests
