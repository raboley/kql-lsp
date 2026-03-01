# Testing Schema Features in IntelliJ (Slices 21-25)

## Prerequisites

1. Build the LSP binary:
   ```bash
   cd lsp && cargo build --release
   ```
2. Build the IntelliJ plugin:
   ```bash
   cd intellij && ./gradlew build
   ```
3. Start the IDE sandbox:
   ```bash
   cd intellij && ./gradlew runIdeForUiTests
   ```
   Wait for IntelliJ to fully open the `test-project/` directory.

## How Schema Loading Works

The test project already has two config files:

- `intellij/test-project/.kql-lsp.json` — tells the LSP where to find the schema
- `intellij/test-project/.kql-schema.json` — defines two tables: **StormEvents** (5 columns) and **PopulationData** (2 columns)

When the LSP starts, it reads `.kql-lsp.json` from the project root and loads the schema automatically. No manual configuration needed.

## What to Test

### 1. Table Name Completion (Slice 21+22)

Open or create a `.kql` file in the test project. On an empty line, start typing:

```
Storm
```

Press `Ctrl+Space` (or wait for auto-complete). You should see:
- **StormEvents** (with "Table" detail)
- **PopulationData** (with "Table" detail)

### 2. Column Name Completion (Slice 23)

Type a query with a pipe and tabular operator:

```
StormEvents | where
```

Place cursor after `where ` and press `Ctrl+Space`. You should see column names:
- **StartTime** (datetime)
- **EndTime** (datetime)
- **State** (string)
- **EventType** (string)
- **DamageProperty** (long)

Also works after `project`, `extend`, `distinct`, and `by`:

```
StormEvents | project
StormEvents | summarize count() by
```

### 3. Schema Diagnostics (Slice 24)

Type a query with an unknown table name:

```
FakeTable | take 10
```

You should see a **warning** underline on `FakeTable` with the message: *Unknown table 'FakeTable'*

Now try an unknown column:

```
StormEvents | where BogusColumn > 5
```

You should see a **warning** underline on `BogusColumn` with: *Unknown column 'BogusColumn' in table 'StormEvents'*

Known tables and columns should have **no warnings**:

```
StormEvents | where State == "TX"
```

Note: Warnings (not errors) because the schema is loaded from a static file. Live ADX schema would produce errors.

### 4. Schema Hover (Slice 25)

Hover over a **table name**:

```
StormEvents | take 10
```

Hover over `StormEvents` — you should see a tooltip listing all columns with their types.

Hover over a **column name**:

```
StormEvents | where State == "TX"
```

Hover over `State` — you should see: `State: string (column in StormEvents)`

## Available Tables and Columns

From `.kql-schema.json`:

| Table | Column | Type |
|-------|--------|------|
| StormEvents | StartTime | datetime |
| StormEvents | EndTime | datetime |
| StormEvents | State | string |
| StormEvents | EventType | string |
| StormEvents | DamageProperty | long |
| PopulationData | State | string |
| PopulationData | Population | long |

## Using Your Own Schema

To test with different tables, edit `intellij/test-project/.kql-schema.json`. The format is:

```json
{
  "database": "YourDB",
  "tables": [
    {
      "name": "YourTable",
      "columns": [
        { "name": "ColumnName", "type": "string" }
      ]
    }
  ]
}
```

After editing, restart the LSP (close and reopen the IDE sandbox, or restart the LSP server via lsp4ij settings).
