//! JSON serialization for documentation graphs.

use crate::model::DocPackage;

/// Serialize documentation package to formatted JSON string.
pub fn generate_json(package: &DocPackage) -> serde_json::Result<String> {
    serde_json::to_string_pretty(package)
}
