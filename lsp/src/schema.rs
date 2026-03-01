//! Schema types for database tables and columns.

use std::collections::HashMap;
use std::path::Path;

/// How the schema was obtained — determines diagnostic severity.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaSource {
    /// From a live ADX cluster — diagnostics are Errors
    Live,
    /// From a static JSON file — diagnostics are Warnings
    Static,
    /// No schema loaded — no schema diagnostics
    None,
}

/// A column in a table.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
}

/// A table in a database.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

/// A database schema containing tables.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DatabaseSchema {
    pub database: String,
    pub tables: Vec<Table>,
}

/// Schema store attached to ServerState.
pub struct SchemaStore {
    pub source: SchemaSource,
    schema: Option<DatabaseSchema>,
    /// Fast lookup: lowercase table name -> index in schema.tables
    table_index: HashMap<String, usize>,
}

impl SchemaStore {
    pub fn new() -> Self {
        Self {
            source: SchemaSource::None,
            schema: None,
            table_index: HashMap::new(),
        }
    }

    pub fn load(&mut self, schema: DatabaseSchema, source: SchemaSource) {
        let mut index = HashMap::new();
        for (i, table) in schema.tables.iter().enumerate() {
            index.insert(table.name.to_lowercase(), i);
        }
        self.table_index = index;
        self.schema = Some(schema);
        self.source = source;
    }

    pub fn table_names(&self) -> Vec<&str> {
        match &self.schema {
            Some(s) => s.tables.iter().map(|t| t.name.as_str()).collect(),
            None => vec![],
        }
    }

    pub fn has_table(&self, name: &str) -> bool {
        self.table_index.contains_key(&name.to_lowercase())
    }

    pub fn columns_for_table(&self, table: &str) -> Vec<&Column> {
        if let Some(idx) = self.table_index.get(&table.to_lowercase()) {
            if let Some(schema) = &self.schema {
                return schema.tables[*idx].columns.iter().collect();
            }
        }
        vec![]
    }

    pub fn has_column(&self, table: &str, column: &str) -> bool {
        let col_lower = column.to_lowercase();
        self.columns_for_table(table)
            .iter()
            .any(|c| c.name.to_lowercase() == col_lower)
    }

    pub fn is_loaded(&self) -> bool {
        self.schema.is_some()
    }
}

/// Load a DatabaseSchema from a JSON file.
pub fn load_from_file(path: &Path) -> Result<DatabaseSchema, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read schema file {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse schema file {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schema() -> DatabaseSchema {
        serde_json::from_str(r#"{
            "database": "TestDB",
            "tables": [
                {
                    "name": "StormEvents",
                    "columns": [
                        { "name": "State", "type": "string" },
                        { "name": "DamageProperty", "type": "long" }
                    ]
                },
                {
                    "name": "PopulationData",
                    "columns": [
                        { "name": "State", "type": "string" },
                        { "name": "Population", "type": "long" }
                    ]
                }
            ]
        }"#).unwrap()
    }

    #[test]
    fn table_names() {
        let mut store = SchemaStore::new();
        store.load(test_schema(), SchemaSource::Static);
        let names = store.table_names();
        assert!(names.contains(&"StormEvents"));
        assert!(names.contains(&"PopulationData"));
    }

    #[test]
    fn has_table_case_insensitive() {
        let mut store = SchemaStore::new();
        store.load(test_schema(), SchemaSource::Static);
        assert!(store.has_table("StormEvents"));
        assert!(store.has_table("stormevents"));
        assert!(!store.has_table("NonExistent"));
    }

    #[test]
    fn columns_for_table() {
        let mut store = SchemaStore::new();
        store.load(test_schema(), SchemaSource::Static);
        let cols = store.columns_for_table("StormEvents");
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "State");
        assert_eq!(cols[1].name, "DamageProperty");
    }

    #[test]
    fn has_column_case_insensitive() {
        let mut store = SchemaStore::new();
        store.load(test_schema(), SchemaSource::Static);
        assert!(store.has_column("StormEvents", "State"));
        assert!(store.has_column("stormevents", "state"));
        assert!(!store.has_column("StormEvents", "FakeColumn"));
    }

    #[test]
    fn empty_store() {
        let store = SchemaStore::new();
        assert!(!store.is_loaded());
        assert!(store.table_names().is_empty());
        assert!(!store.has_table("anything"));
    }
}
