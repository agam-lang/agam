//! SARIF (Static Analysis Results Interchange Format) 2.1.0 exporter for Agam diagnostics.

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, DiagnosticLevel};
use crate::span::SourceFile;

/// Root SARIF 2.1.0 log document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

/// A run of the agamc tool producing analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

/// Description of the analysis tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

/// Driver descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    #[serde(rename = "informationUri")]
    pub information_uri: String,
    pub version: String,
    pub rules: Vec<SarifRule>,
}

/// A diagnostic rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
    #[serde(rename = "helpUri", skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

/// A single diagnostic finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId", skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    pub level: String,
    pub message: SarifMessage,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<SarifLocation>,
}

/// Location in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

/// Physical file locus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

/// File path URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

/// 1-indexed line and column region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(rename = "startColumn")]
    pub start_column: usize,
    #[serde(rename = "endLine")]
    pub end_line: usize,
    #[serde(rename = "endColumn")]
    pub end_column: usize,
}

/// Human-readable text message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifMessage {
    pub text: String,
}

/// Convert a list of Agam diagnostics to a SARIF 2.1.0 log document.
pub fn to_sarif(diagnostics: &[Diagnostic], source_file: Option<&SourceFile>) -> SarifLog {
    let mut results = Vec::new();
    let mut rules = Vec::new();
    let mut seen_rules = std::collections::HashSet::new();

    for diag in diagnostics {
        let rule_id = diag.code.map(|c| {
            if seen_rules.insert(c.0) {
                rules.push(SarifRule {
                    id: c.0.to_string(),
                    name: format!("Agam{}", c.0),
                    short_description: SarifMessage {
                        text: diag.message.clone(),
                    },
                    help_uri: Some(format!("https://agam-lang.org/errors/{}", c.0)),
                });
            }
            c.0.to_string()
        });

        let level = match diag.level {
            DiagnosticLevel::Error | DiagnosticLevel::Ice => "error".to_string(),
            DiagnosticLevel::Warning => "warning".to_string(),
            DiagnosticLevel::Note => "note".to_string(),
        };

        let mut locations = Vec::new();
        for label in &diag.labels {
            let (start_line, start_col, end_line, end_col) = if let Some(sf) = source_file {
                let (s_line, s_col) = sf.offset_to_line_col(label.span.start as usize);
                let (e_line, e_col) = sf.offset_to_line_col(label.span.end as usize);
                (s_line + 1, s_col + 1, e_line + 1, e_col + 1)
            } else {
                (1, 1, 1, 1)
            };

            let uri = source_file
                .map(|sf| sf.path.clone())
                .unwrap_or_else(|| "source.agam".to_string());

            locations.push(SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation { uri },
                    region: SarifRegion {
                        start_line,
                        start_column: start_col,
                        end_line,
                        end_column: end_col,
                    },
                },
            });
        }

        results.push(SarifResult {
            rule_id,
            level,
            message: SarifMessage {
                text: diag.message.clone(),
            },
            locations,
        });
    }

    SarifLog {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "agamc".to_string(),
                    information_uri: "https://agam-lang.org".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    rules,
                },
            },
            results,
        }],
    }
}

/// Convert a list of diagnostics to a formatted JSON SARIF string.
pub fn to_sarif_json(diagnostics: &[Diagnostic], source_file: Option<&SourceFile>) -> String {
    let sarif = to_sarif(diagnostics, source_file);
    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Label;
    use crate::span::{SourceId, Span};

    #[test]
    fn test_sarif_export() {
        let source_file = SourceFile::new(
            SourceId(0),
            "test.agam".to_string(),
            "let x: i32 = \"hello\";\n".to_string(),
        );
        let mut diag = Diagnostic::error("E0308", "mismatched types: expected i32, found str");
        diag.labels.push(Label::primary(
            Span::new(SourceId(0), 13, 20),
            "expected i32, found str",
        ));

        let sarif = to_sarif(&[diag], Some(&source_file));
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        let run = &sarif.runs[0];
        assert_eq!(run.tool.driver.name, "agamc");
        assert_eq!(run.results.len(), 1);
        assert_eq!(run.results[0].rule_id, Some("E0308".to_string()));
        assert_eq!(run.results[0].level, "error");
        assert_eq!(run.results[0].locations.len(), 1);

        let json = to_sarif_json(
            &[Diagnostic::warning("W0001", "unused variable")],
            Some(&source_file),
        );
        assert!(json.contains("W0001"));
        assert!(json.contains("warning"));
    }
}
