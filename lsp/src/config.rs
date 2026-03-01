//! LSP configuration parsing from initializationOptions or .kql-lsp.json.

use serde_json::Value;
use std::path::{Path, PathBuf};

/// ADX cluster connection settings.
#[derive(Debug, Clone)]
pub struct AdxConfig {
    pub cluster: String,
    pub database: String,
}

/// LSP configuration.
#[derive(Debug, Clone, Default)]
pub struct LspConfig {
    pub schema_file: Option<PathBuf>,
    pub adx: Option<AdxConfig>,
}

impl LspConfig {
    /// Parse config from LSP initializationOptions JSON value.
    pub fn from_init_options(opts: &Value, root_dir: Option<&Path>) -> Self {
        let mut config = LspConfig::default();

        if let Some(sf) = opts.get("schemaFile").and_then(|v| v.as_str()) {
            let path = PathBuf::from(sf);
            config.schema_file = Some(if path.is_absolute() {
                path
            } else if let Some(root) = root_dir {
                root.join(path)
            } else {
                path
            });
        }

        if let Some(adx) = opts.get("adx") {
            let cluster = adx.get("cluster").and_then(|v| v.as_str());
            let database = adx.get("database").and_then(|v| v.as_str());
            if let (Some(c), Some(d)) = (cluster, database) {
                config.adx = Some(AdxConfig {
                    cluster: c.to_string(),
                    database: d.to_string(),
                });
            }
        }

        config
    }

    /// Try to load config from a .kql-lsp.json file in the given directory.
    pub fn from_file(dir: &Path) -> Option<Self> {
        let config_path = dir.join(".kql-lsp.json");
        let content = std::fs::read_to_string(&config_path).ok()?;
        let value: Value = serde_json::from_str(&content).ok()?;
        Some(Self::from_init_options(&value, Some(dir)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_schema_file_absolute() {
        let opts = serde_json::json!({ "schemaFile": "/path/to/schema.json" });
        let config = LspConfig::from_init_options(&opts, None);
        assert_eq!(config.schema_file.unwrap(), PathBuf::from("/path/to/schema.json"));
    }

    #[test]
    fn parse_schema_file_relative_with_root() {
        let opts = serde_json::json!({ "schemaFile": ".kql-schema.json" });
        let config = LspConfig::from_init_options(&opts, Some(Path::new("/project")));
        assert_eq!(config.schema_file.unwrap(), PathBuf::from("/project/.kql-schema.json"));
    }

    #[test]
    fn parse_adx_config() {
        let opts = serde_json::json!({
            "adx": {
                "cluster": "https://help.kusto.windows.net",
                "database": "Samples"
            }
        });
        let config = LspConfig::from_init_options(&opts, None);
        let adx = config.adx.unwrap();
        assert_eq!(adx.cluster, "https://help.kusto.windows.net");
        assert_eq!(adx.database, "Samples");
    }

    #[test]
    fn parse_empty_options() {
        let opts = serde_json::json!({});
        let config = LspConfig::from_init_options(&opts, None);
        assert!(config.schema_file.is_none());
        assert!(config.adx.is_none());
    }
}
