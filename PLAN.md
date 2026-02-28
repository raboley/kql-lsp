# Production-Grade KQL LSP Development Plan

## Context

Build a full-featured LSP for Microsoft's Kusto Query Language (KQL) in Rust, with plugins for Neovim and IntelliJ. The current codebase has a working LSP skeleton (stdio loop, initialize/shutdown, empty diagnostics) with both editor plugins scaffolded and tested.

## Core Principle: Vertical Feature Slices

Every slice delivers a thin, end-to-end feature: write a Neovim test, implement in Rust, verify in Neovim, write an IntelliJ test, verify in IntelliJ, commit. **No slice is done until both editors prove the feature works.** The parser, lexer, and infrastructure grow incrementally to support each slice -- we never build a complete horizontal layer and verify it later.

## Architecture Decisions

| Area | Choice | Rationale |
|------|--------|-----------|
| **Rope** | `ropey 1.6` | Most mature Rust rope; built-in UTF-16 conversion for LSP protocol |
| **LSP transport** | Keep `rpc.rs` + add `lsp-types 0.97` | Current transport works; `lsp-types` adds typed structs on top |
| **Parser** | Hand-written recursive descent | Error recovery at `|` sync points; proven by rust-analyzer |
| **Syntax trees** | `rowan 0.15` (lossless CST) | Preserves all text; enables formatting, refactoring, precise diagnostics |
| **No async runtime** | Synchronous stdio loop | Simple, sufficient, debuggable |

## Target Module Structure (grows incrementally with each slice)

```
lsp/src/
  main.rs, rpc.rs, server.rs, document.rs, syntax.rs
  diagnostics.rs, semantic_tokens.rs, symbols.rs, hover.rs
  definition.rs, references.rs, code_actions.rs, formatting.rs
  signature_help.rs, folding.rs, rename.rs, inlay_hints.rs
  lexer/ { mod.rs, token.rs, lexer.rs }
  parser/ { mod.rs, parser.rs, grammar/ { statements.rs, operators.rs, expressions.rs, commands.rs } }
  analysis/ { mod.rs, symbols.rs, scope.rs }
  completion/ { mod.rs, context.rs, providers/ { keywords.rs, functions.rs, operators.rs } }
  builtins/ { mod.rs, functions.rs, operators.rs }
```

---

## Slice 1: Rope Foundation + Document Store Verification

**What ships**: Documents stored in rope data structure; UTF-16 position conversion working.

**Cargo.toml**: Add `ropey = "1.6"`, `lsp-types = "0.97"`

**New files**: `document.rs`, `server.rs`

**Rust implementation**:
- `Document` struct with `uri`, `version`, `rope: Rope`, `language_id`
- `DocumentStore` with open/change/close/get + `offset_to_position` / `position_to_offset` (UTF-16 conversion)
- `ServerState` holding `DocumentStore`
- Refactor `main.rs` to use `ServerState` and `lsp-types` for typed deserialization

**Neovim test** (`neovim/test/document_spec.lua`):
- Open a .kql buffer with multi-line content including non-ASCII characters
- Verify LSP client attaches and the server responds to didOpen without error
- Verify diagnostics are published (empty array -- proving the document was stored and processed)

**IntelliJ test**:
- Open test.kql with multi-line content
- Verify LSP initializes and processes the file (log-based or diagnostic verification)

---

## Slice 2: Parse Simple Query + First Real Diagnostic

**What ships**: `StormEvents | take 10` parses clean; `StormEvents | where` produces a diagnostic in both editors.

**Cargo.toml**: Add `rowan = "0.15"`, `pretty_assertions = "1.4"` (dev)

**New files**: `lexer/{mod.rs, token.rs, lexer.rs}`, `parser/{mod.rs, parser.rs}`, `parser/grammar/{mod.rs, statements.rs, operators.rs, expressions.rs}`, `syntax.rs`, `diagnostics.rs`

**Scope of lexer** (just enough for this slice):
- `Identifier`, `IntLiteral`, `Pipe`, `Whitespace`, `Newline`, `Eof`, `Error`
- Keywords: `WhereKw`, `TakeKw`, `LimitKw`
- Operators: `GreaterThan`, `LessThan`, `EqualEqual`, `Equals`

**Scope of parser** (just enough for this slice):
- `SourceFile` -> `QueryStatement` -> `PipeExpression`
- Tabular operators: `TakeClause` (take + number), `WhereClause` (where + expression -- expression is just a minimal binary expr for now)
- Error recovery: `| where` with no predicate produces `ErrorNode` + `ParseError`

**Diagnostics**: Convert `ParseError` -> `lsp_types::Diagnostic` with range from rope position conversion

**Neovim test** (`neovim/test/diagnostics_spec.lua`):
- `StormEvents | where` produces at least one ERROR diagnostic
- `StormEvents | take 10` produces zero diagnostics
- Editing from invalid to valid clears diagnostics

**IntelliJ test**:
- Open file with `StormEvents | where` -- verify error highlighting appears in editor
- Open file with `StormEvents | take 10` -- verify no error highlighting

---

## Slice 3: Semantic Tokens for Simple Queries

**What ships**: Keywords, identifiers, numbers, and pipe operators get semantic coloring in both editors.

**Extend lexer**: Add `Comment`, `StringLiteral`, `RealLiteral`, `BoolLiteral`

**New file**: `semantic_tokens.rs`

**Token mapping** (just what we can produce so far):
- `WhereKw`, `TakeKw` -> `keyword`
- `Identifier` -> `property` (column/table references)
- `IntLiteral`, `RealLiteral` -> `number`
- `StringLiteral` -> `string`
- `BoolLiteral` -> `keyword` (or custom)
- `Pipe` -> `operator`
- `Comment` -> `comment`

**Register capability**: `semanticTokensProvider` with legend + `full: true`

**Neovim test** (`neovim/test/semantic_tokens_spec.lua`):
- Request `textDocument/semanticTokens/full` for `StormEvents | take 10`
- Verify non-empty response with correct number of tokens
- Verify keyword token appears for `take`, number token for `10`

**IntelliJ test**:
- Open .kql file, verify semantic highlighting is active (lsp4ij auto-requests this)
- Can verify via editor color scheme inspection or log-based verification

---

## Slice 4: Document Symbols for Let + Query

**What ships**: Outline view shows let bindings and query statements in both editors.

**Extend parser**: Parse `let x = <expr>;` statements (scalar let, expression is still minimal)

**Extend lexer**: Add `LetKw`, `Semicolon`

**New file**: `symbols.rs`

**Symbol mapping**:
- `LetStatement` -> `Variable` symbol with name
- `QueryStatement` -> `Event` symbol with first table name

**Register capability**: `documentSymbolProvider: true`

**Neovim test** (`neovim/test/symbols_spec.lua`):
- Open buffer with `let threshold = 100;\nStormEvents | take 10`
- Request `textDocument/documentSymbol`
- Verify 2 symbols: "threshold" (Variable) and "StormEvents" (query)

**IntelliJ test**:
- Open file with let + query
- Verify Structure tool window shows symbols (via Remote Robot)

---

## Slice 5: Completion After Pipe

**What ships**: Typing `|` shows tabular operator completions in both editors.

**New files**: `completion/{mod.rs, context.rs}`, `completion/providers/{mod.rs, keywords.rs}`, `builtins/{mod.rs, operators.rs}`

**Completion context**: Detect cursor is after `|` -> offer tabular operator keywords (`where`, `project`, `extend`, `summarize`, `take`, `top`, `sort`, `count`, `distinct`, `join`, `union`)

**Register capability**: `completionProvider` with trigger char `|`

**Neovim test** (`neovim/test/completion_spec.lua`):
- Open buffer with `StormEvents | `
- Request `textDocument/completion` at end of line
- Verify response contains `where`, `summarize`, `project`, `take`

**IntelliJ test**:
- Type `StormEvents | ` and trigger completion
- Verify completion popup shows tabular operators

---

## Slice 6: `where` with Comparison Expressions + Diagnostics

**What ships**: `| where Column > 5` parses correctly; `| where Column >` (incomplete) shows diagnostic.

**Extend parser**:
- Binary expressions: arithmetic (`+`, `-`, `*`, `/`, `%`), comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Operator precedence
- Column references as identifiers

**Extend lexer**: Add `Plus`, `Minus`, `Star`, `Slash`, `Percent`, `NotEqual`, `LessEqual`, `GreaterEqual`

**Neovim test**:
- `StormEvents | where DamageProperty > 100` -> zero diagnostics
- `StormEvents | where DamageProperty >` -> ERROR diagnostic
- `StormEvents | where DamageProperty > 100 | take 10` -> zero diagnostics (multi-pipe)

**IntelliJ test**: Same verification of diagnostics in editor

---

## Slice 7: `project` and `extend` Operators

**What ships**: `| project Col1, Col2` and `| extend NewCol = expr` parse and highlight.

**Extend parser**: `ProjectClause` (comma-separated column list with optional rename), `ExtendClause` (assignment list)

**Extend lexer**: `ProjectKw`, `ExtendKw`, `Comma`

**Update semantic tokens**: Column names in project/extend

**Update completion**: After `| project ` and `| extend `, offer relevant completions

**Neovim test**:
- `StormEvents | project State, EventType` -> zero diagnostics, correct symbols
- `StormEvents | extend Duration = EndTime - StartTime` -> zero diagnostics

**IntelliJ test**: Verify highlighting and no false diagnostics

---

## Slice 8: `summarize` with Aggregation Functions + Function Completion

**What ships**: `| summarize count() by State` works; function completions appear inside summarize.

**Extend parser**: `SummarizeClause` with `by` group-by, `FunctionCallExpr` (name + parenthesized args)

**Extend lexer**: `SummarizeKw`, `ByKw`, `LParen`, `RParen`

**New files**: `builtins/functions.rs` (start with ~15 core functions: `count`, `sum`, `avg`, `min`, `max`, `dcount`, `countif`, `sumif`, `ago`, `now`, `strcat`, `tostring`, `toint`, `tolong`, `strlen`)

**Extend completion**: In summarize context -> offer aggregation functions (`count`, `sum`, `avg`, etc.)

**New file**: `completion/providers/functions.rs`

**Neovim test**:
- `StormEvents | summarize count() by State` -> zero diagnostics
- Completion inside summarize returns aggregation functions
- `StormEvents | summarize` (incomplete) -> appropriate diagnostic

**IntelliJ test**: Verify summarize parses, completions appear

---

## Slice 9: Hover for Built-in Functions

**What ships**: Hovering over `count()`, `ago()`, etc. shows documentation in both editors.

**New file**: `hover.rs`

**Implementation**: Find token at cursor position in CST, look up in builtins catalog, format as Markdown with signature and description

**Neovim test** (`neovim/test/hover_spec.lua`):
- Hover over `count` in `| summarize count()` -> shows "count() -> long" with description
- Hover over `ago` in `ago(1h)` -> shows "ago(timespan) -> datetime"
- Hover over `StormEvents` (table name) -> returns null/empty (no catalog for tables yet)

**IntelliJ test**: Hover over function, verify documentation popup

---

## Slice 10: String Operators + Timespan Literals

**What ships**: `| where Name contains "test"` and `ago(1h)` work correctly with highlighting.

**Extend lexer**: String operator keywords (`ContainsKw`, `HasKw`, `StartswithKw`, `EndswithKw`, `MatchesRegexKw`, `InKw` + `_cs` and `not` variants), `TimespanLiteral`

**Extend parser**: String operators as binary expression operators, timespan literals in expressions

**Update semantic tokens**: String operators colored as `operator`, timespan literals as `number`

**Neovim test**:
- `StormEvents | where State contains "TEX"` -> zero diagnostics
- `StormEvents | where StartTime > ago(1h)` -> zero diagnostics (timespan literal works)
- Semantic tokens correctly classify `contains` and `1h`

**IntelliJ test**: Verify parsing + highlighting of string operators and timespan literals

---

## Slice 11: Let Bindings + Go-to-Definition

**What ships**: Go-to-definition on a let-bound variable jumps to its declaration in both editors.

**New files**: `analysis/{mod.rs, symbols.rs, scope.rs}`, `definition.rs`

**Extend parser**: Full let statement support (scalar, tabular: `let x = T | where ...;`)

**Implementation**: Build symbol table from CST, scope resolution for let bindings, `textDocument/definition` handler

**Register capability**: `definitionProvider: true`

**Neovim test** (`neovim/test/definition_spec.lua`):
- `let threshold = 100;\nStormEvents | where Damage > threshold` -- go-to-definition on `threshold` in where clause jumps to line 0
- Go-to-definition on `StormEvents` returns null (no table catalog)

**IntelliJ test**: Ctrl+Click on let-bound variable navigates to definition

---

## Slice 12: Find References

**What ships**: Find all references shows every usage of a let-bound variable.

**New file**: `references.rs`

**Register capability**: `referencesProvider: true`

**Neovim test**:
- `let x = 10;\nlet y = x + 1;\nT | where Col > x` -- find references on `x` returns 3 locations

**IntelliJ test**: Right-click -> Find Usages shows all references

---

## Slice 13: `join` Operator

**What ships**: `T1 | join kind=inner (T2) on Key` parses with correct highlighting and diagnostics.

**Extend lexer**: `JoinKw`, `OnKw`, `KindKw`, join kind keywords (`InnerKw`, `LeftouterKw`, etc.), `DollarSign`

**Extend parser**: `JoinClause` with kind, parenthesized right-side expression, on-condition, `$left.$right` references

**Extend completion**: After `join kind=` offer join kinds

**Neovim test**:
- `T1 | join kind=inner (T2) on Key` -> zero diagnostics
- `T1 | join kind=` -> completion shows join kinds

**IntelliJ test**: Verify join parsing and completion

---

## Slice 14: Management Commands (`.show`, `.create`)

**What ships**: `.show tables`, `.create table T (Col: string)` parse with highlighting; diagnostics for malformed commands.

**Extend lexer**: `Dot`, `Colon`

**New file**: `parser/grammar/commands.rs`

**Extend parser**: `ManagementCommand` starting with `.`, loose parsing of command arguments (`.show` + identifiers, `.create table` + schema)

**Update symbols**: Management commands appear in outline

**Neovim test**:
- `.show tables` -> zero diagnostics, appears in document symbols
- `.create table MyTable (Name: string, Age: int)` -> zero diagnostics

**IntelliJ test**: Verify management commands parse and highlight

---

## Slice 15: Multiple Statements + Statement Separation

**What ships**: Files with multiple queries separated by blank lines parse independently; let chains with semicolons work.

**Extend parser**: Statement boundary detection at blank lines, multiple `QueryStatement` nodes in `SourceFile`

**Neovim test**:
- File with valid query, blank line, invalid query -> diagnostic only on the invalid query (correct range)
- `let x = 1;\nlet y = 2;\nT | where Col > x` -> all three statements parse

**IntelliJ test**: Multiple statement file parses with independent diagnostics

---

## Slice 16: Incremental Text Sync (Partial Updates)

**What ships**: Switch from full document sync to incremental sync. Editors send only the changed text range instead of the entire document on every keystroke, dramatically improving performance for large files.

**Change `textDocumentSync`**: From `1` (Full) to `2` (Incremental) in initialize capabilities

**Extend `DocumentStore`**: Add `change_incremental(&mut self, uri, version, changes)` that applies `TextDocumentContentChangeEvent` ranges to the rope using `rope.remove()` and `rope.insert()` with UTF-16 position conversion

**Key implementation**:
- Parse `contentChanges` array with `range` (start/end line+character) and `text`
- Convert LSP positions to rope char offsets using `position_to_offset`
- Apply changes in reverse order (largest offset first) to preserve positions
- Fall back to full replacement if a change has no range

**Neovim test** (`neovim/test/incremental_sync_spec.lua`):
- Open a multi-line document, type a single character mid-document
- Verify only an incremental change was sent (not the full document)
- Verify diagnostics still work after incremental edits
- Verify rapid sequential edits (simulating fast typing) don't corrupt the document

**IntelliJ test**:
- Open a file, make an edit, verify the LSP processes the change
- Verify document content remains consistent after multiple edits

---

## Slice 17: Signature Help for Functions

**What ships**: Typing inside `ago(` shows parameter info in both editors.

**New file**: `signature_help.rs`

**Implementation**: Detect cursor inside function call parens, find which parameter is active (count commas), look up function in builtins catalog

**Register capability**: `signatureHelpProvider` with trigger chars `(`, `,`

**Neovim test**:
- Position cursor inside `ago(` -> signature help shows `ago(a: timespan) -> datetime`
- Position cursor after comma in `strcat("a", ` -> shows second parameter active

**IntelliJ test**: Verify parameter info popup appears

---

## Slice 18: Code Action -- Fix Missing Semicolon

**What ships**: When a let statement is missing its semicolon, a quick fix offers to add it.

**New file**: `code_actions.rs`

**Register capability**: `codeActionProvider: true`

**Neovim test**:
- `let x = 10\nT | take 5` (missing semicolon) -> code action "Add missing semicolon" offered
- Apply the action -> semicolon is inserted, diagnostic clears

**IntelliJ test**: Alt+Enter on diagnostic shows quick fix

---

## Slice 19: Formatting

**What ships**: Auto-format normalizes pipe chain indentation and spacing.

**New file**: `formatting.rs`

**Rules**: Pipe operators on new lines with consistent indent, spaces around binary operators, lowercase keywords

**Register capabilities**: `documentFormattingProvider`, `documentRangeFormattingProvider`

**Neovim test**:
- Input: `StormEvents|where X>5|take 10`
- After format: `StormEvents\n| where X > 5\n| take 10` (or similar style)

**IntelliJ test**: Ctrl+Alt+L formats the document

---

## Slice 20: Folding Ranges

**What ships**: Multi-line queries and let blocks can be collapsed.

**New file**: `folding.rs`

**Register capability**: `foldingRangeProvider: true`

**Neovim test**: Multi-line query returns a folding range spanning its lines

**IntelliJ test**: Folding icons appear in gutter for multi-line queries

---

## Slice 21: Rename Symbol

**What ships**: Rename a let-bound variable and all references update.

**New file**: `rename.rs`

**Register capability**: `renameProvider` with `prepareProvider: true`

**Neovim test**:
- Rename `threshold` -> `minDamage` in `let threshold = 100;\nT | where D > threshold`
- Both occurrences update

**IntelliJ test**: Shift+F6 rename works across references

---

## Slice 22+: Incremental Expansion

After the core slices above, expand depth in each area:

- **More tabular operators**: `union`, `sort`, `top`, `distinct`, `mv-expand`, `mv-apply`, `parse`, `evaluate`, `render`, `make-series`, `serialize`, `scan`, `find`, `search`, `lookup`, `externaldata`, `datatable`, `range`, `print` -- each one is its own mini-slice (parse + highlight + complete + test in both editors)
- **More built-in functions**: Expand catalog from ~15 to 200+ (string, datetime, numeric, dynamic, geo, hash categories)
- **More string literal types**: Verbatim `@"..."`, obfuscated `h@"..."`, multi-line
- **datetime/dynamic/guid literals**: `datetime(2023-01-01)`, `dynamic([1,2,3])`, `guid(...)`
- **Logical operators**: `and`, `or`, `not`

- **Inlay hints**: Inferred types, parameter names
- **Selection range**: Smart expand/shrink based on CST
- **Semantic token deltas**: `semanticTokens/full/delta` for performance
- **More code actions**: Suggest `=~` for `==`, wrap column names in brackets, extract sub-query
- **Workspace symbols**: Cross-file let-binding search
- **More management commands**: Typed parsing for `.alter`, `.drop`, `.set`, policy commands, etc.

Each of these is a small vertical slice: extend the relevant layer (lexer/parser/analysis/LSP handler), write Neovim test, implement, verify Neovim, write IntelliJ test, verify IntelliJ, commit.

---

## Verification Strategy

Every slice follows the TDD workflow from `DEVELOPMENT.md`:

1. **Write failing Neovim test** (~2-3 sec)
2. **Implement in Rust** (cargo build + cargo test)
3. **Neovim test passes**
4. **Write IntelliJ UI test** (~20 sec)
5. **IntelliJ test passes**
6. **Commit**

A feature is **NOT done** until it works in both editors.

```powershell
cd lsp && cargo build --release          # Build
cd lsp && cargo test                     # Unit tests
nvim --headless ... -c "PlenaryBustedFile neovim/test/<spec>.lua"  # Neovim
cd intellij && ./gradlew uiTest         # IntelliJ
```

## Critical Files

- `lsp/src/main.rs` -- Message dispatch, extended each slice
- `lsp/src/rpc.rs` -- Transport layer (stable)
- `lsp/Cargo.toml` -- Dependencies added in Slices 1-2
- `neovim/test/helpers.lua` -- New request helpers per feature
- `intellij/src/test/kotlin/com/kqllsp/ui/LspIntegrationTest.kt` -- New test per feature

## Risks

| Risk | Mitigation |
|------|-----------|
| No formal KQL grammar | Start small, grow incrementally; error recovery handles unknown syntax |
| UTF-16 offset bugs | Unit test in Slice 1 before anything depends on it |
| Management commands are vast | Parse loosely first, add typed parsing per-command as separate slices |
| Built-in function catalog (200+) | Start with ~15, expand incrementally |
| IntelliJ UI test flakiness | Generous timeouts; IntelliJ is the gate, Neovim is the fast loop |
