//! Domain-Specific Language (DSL) Compiler Extensions (Neural Networks & Pipelines).

use serde::{Deserialize, Serialize};

/// High-level neural network layer definitions in the `@nn` DSL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NnDslLayer {
    Conv2d {
        in_channels: u32,
        out_channels: u32,
        kernel_size: u32,
    },
    Linear {
        in_features: u32,
        out_features: u32,
    },
    Relu,
    Gelu,
    MaxPool2d {
        kernel_size: u32,
    },
    Softmax,
}

/// Parse `@nn { conv2d(3, 64, 3) -> relu -> pool(2) -> linear(1024, 10) }`
pub fn parse_nn_dsl(input: &str) -> Result<Vec<NnDslLayer>, String> {
    let mut layers = Vec::new();
    let parts: Vec<&str> = input.split("->").map(|s| s.trim()).collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        if part.starts_with("conv2d") {
            let inner = extract_parentheses_args(part)?;
            let nums = parse_comma_separated_u32(&inner)?;
            if nums.len() != 3 {
                return Err("conv2d requires (in_channels, out_channels, kernel_size)".into());
            }
            layers.push(NnDslLayer::Conv2d {
                in_channels: nums[0],
                out_channels: nums[1],
                kernel_size: nums[2],
            });
        } else if part.starts_with("linear") {
            let inner = extract_parentheses_args(part)?;
            let nums = parse_comma_separated_u32(&inner)?;
            if nums.len() != 2 {
                return Err("linear requires (in_features, out_features)".into());
            }
            layers.push(NnDslLayer::Linear {
                in_features: nums[0],
                out_features: nums[1],
            });
        } else if part == "relu" {
            layers.push(NnDslLayer::Relu);
        } else if part == "gelu" {
            layers.push(NnDslLayer::Gelu);
        } else if part.starts_with("pool") || part.starts_with("maxpool") {
            let inner = extract_parentheses_args(part)?;
            let nums = parse_comma_separated_u32(&inner)?;
            let k = nums.first().copied().unwrap_or(2);
            layers.push(NnDslLayer::MaxPool2d { kernel_size: k });
        } else if part == "softmax" {
            layers.push(NnDslLayer::Softmax);
        } else {
            return Err(format!("Unknown NN DSL layer '{part}'"));
        }
    }

    Ok(layers)
}

/// Emit Agam struct and forward pass implementation for an `@nn` pipeline.
pub fn emit_nn_model_definition(model_name: &str, layers: &[NnDslLayer]) -> String {
    let mut out = format!("struct {} {{\n", model_name);
    for (i, layer) in layers.iter().enumerate() {
        match layer {
            NnDslLayer::Conv2d {
                in_channels,
                out_channels,
                kernel_size,
            } => {
                out.push_str(&format!(
                    "    layer_{i}: Conv2d<{in_channels}, {out_channels}, {kernel_size}>,\n"
                ));
            }
            NnDslLayer::Linear {
                in_features,
                out_features,
            } => {
                out.push_str(&format!(
                    "    layer_{i}: Linear<{in_features}, {out_features}>,\n"
                ));
            }
            _ => {}
        }
    }
    out.push_str("}\n\n");

    out.push_str(&format!("impl {} {{\n", model_name));
    out.push_str("    fn forward(self, mut x: Tensor) -> Tensor {\n");
    for (i, layer) in layers.iter().enumerate() {
        match layer {
            NnDslLayer::Conv2d { .. } | NnDslLayer::Linear { .. } => {
                out.push_str(&format!("        x = self.layer_{i}.forward(x);\n"));
            }
            NnDslLayer::Relu => {
                out.push_str("        x = x.relu();\n");
            }
            NnDslLayer::Gelu => {
                out.push_str("        x = x.gelu();\n");
            }
            NnDslLayer::MaxPool2d { kernel_size } => {
                out.push_str(&format!("        x = x.max_pool2d({kernel_size});\n"));
            }
            NnDslLayer::Softmax => {
                out.push_str("        x = x.softmax();\n");
            }
        }
    }
    out.push_str("        x\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    out
}

fn extract_parentheses_args(s: &str) -> Result<String, String> {
    let start = s.find('(').ok_or_else(|| format!("Expected '(' in {s}"))?;
    let end = s.rfind(')').ok_or_else(|| format!("Expected ')' in {s}"))?;
    if start >= end {
        return Err(format!("Mismatched parentheses in {s}"));
    }
    Ok(s[start + 1..end].trim().to_string())
}

fn parse_comma_separated_u32(s: &str) -> Result<Vec<u32>, String> {
    let mut nums = Vec::new();
    for item in s.split(',') {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            let n = trimmed
                .parse::<u32>()
                .map_err(|e| format!("Failed to parse number '{trimmed}': {e}"))?;
            nums.push(n);
        }
    }
    Ok(nums)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nn_dsl_parsing_and_emission() {
        let dsl_code = "conv2d(3, 64, 3) -> relu -> pool(2) -> linear(1024, 10)";
        let layers = parse_nn_dsl(dsl_code).expect("parsing succeeds");
        assert_eq!(layers.len(), 4);

        let code = emit_nn_model_definition("SimpleClassifier", &layers);
        assert!(code.contains("struct SimpleClassifier"));
        assert!(code.contains("layer_0: Conv2d<3, 64, 3>"));
        assert!(code.contains("x = self.layer_0.forward(x);"));
        assert!(code.contains("x = x.relu();"));
        assert!(code.contains("x = x.max_pool2d(2);"));
    }
}
