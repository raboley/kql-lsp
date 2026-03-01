# Completed Slices 1–20: Core LSP Features

All slices below are **implemented and tested** in both Neovim and IntelliJ.

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

---

## Slice 3: Semantic Tokens for Simple Queries

**What ships**: Keywords, identifiers, numbers, and pipe operators get semantic coloring in both editors.

---

## Slice 4: Document Symbols for Let + Query

**What ships**: Outline view shows let bindings and query statements in both editors.

---

## Slice 5: Completion After Pipe

**What ships**: Typing `|` shows tabular operator completions in both editors.

---

## Slice 6: `where` with Comparison Expressions + Diagnostics

**What ships**: `| where Column > 5` parses correctly; `| where Column >` (incomplete) shows diagnostic.

---

## Slice 7: `project` and `extend` Operators

**What ships**: `| project Col1, Col2` and `| extend NewCol = expr` parse and highlight.

---

## Slice 8: `summarize` with Aggregation Functions + Function Completion

**What ships**: `| summarize count() by State` works; function completions appear inside summarize.

---

## Slice 9: Hover for Built-in Functions

**What ships**: Hovering over `count()`, `ago()`, etc. shows documentation in both editors.

---

## Slice 10: String Operators + Timespan Literals

**What ships**: `| where Name contains "test"` and `ago(1h)` work correctly with highlighting.

---

## Slice 11: Let Bindings + Go-to-Definition

**What ships**: Go-to-definition on a let-bound variable jumps to its declaration in both editors.

---

## Slice 12: Find References

**What ships**: Find all references shows every usage of a let-bound variable.

---

## Slice 13: `join` Operator

**What ships**: `T1 | join kind=inner (T2) on Key` parses with correct highlighting and diagnostics.

---

## Slice 14: Management Commands (`.show`, `.create`)

**What ships**: `.show tables`, `.create table T (Col: string)` parse with highlighting; diagnostics for malformed commands.

---

## Slice 15: Multiple Statements + Statement Separation

**What ships**: Files with multiple queries separated by blank lines parse independently; let chains with semicolons work.

---

## Slice 16: Incremental Text Sync (Partial Updates)

**What ships**: Switch from full document sync to incremental sync.

---

## Slice 17: Signature Help for Functions

**What ships**: Typing inside `ago(` shows parameter info in both editors.

---

## Slice 18: Code Action -- Fix Missing Semicolon

**What ships**: When a let statement is missing its semicolon, a quick fix offers to add it.

---

## Slice 19: Formatting

**What ships**: Auto-format normalizes pipe chain indentation and spacing.

---

## Slice 20: Folding Ranges

**What ships**: Multi-line queries and let blocks can be collapsed.

---

## Slice 21 (original numbering): Rename Symbol

**What ships**: Rename a let-bound variable and all references update.
