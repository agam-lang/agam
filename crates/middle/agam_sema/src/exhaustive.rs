//! Maranget-style usefulness and exhaustiveness checking for `match`.
//!
//! A row is useful when it matches a value not matched by preceding unguarded
//! rows. Checking every arm finds unreachable arms; a wildcard query produces
//! a missing-pattern witness.

use agam_ast::expr::ExprKind;
use agam_ast::pattern::{Pattern, PatternKind};
use agam_errors::{Diagnostic, Label, NyayaProof, Span};
use std::collections::HashSet;

/// Bounds matrix expansion for hostile or accidentally enormous patterns.
pub const MAX_PATTERN_DEPTH: usize = 128;
pub const MAX_MATCH_ARMS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimplePattern {
    Wildcard,
    Bool(bool),
    Int(i64),
    Str(String),
    Variant(String),
    Tuple(Vec<SimplePattern>),
    Constructor {
        name: String,
        fields: Vec<SimplePattern>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantShape {
    pub name: String,
    pub fields: Vec<TypeShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeShape {
    Bool,
    /// Compatibility form for fieldless enums.
    Enum {
        variants: Vec<String>,
    },
    EnumWithPayload {
        variants: Vec<VariantShape>,
    },
    Int,
    Str,
    Tuple(Vec<TypeShape>),
    Struct {
        name: String,
        fields: Vec<TypeShape>,
    },
    /// Struct layout retaining names for AST-pattern adaptation.
    StructNamed {
        name: String,
        fields: Vec<(String, TypeShape)>,
    },
    Other,
}

/// A guarded arm is checked for reachability but does not cover later arms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternArm {
    pub pattern: SimplePattern,
    pub has_guard: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExhaustivenessError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExhaustivenessReport {
    pub unreachable_arms: Vec<usize>,
    pub missing_witness: Option<SimplePattern>,
    pub diagnostics: Vec<Diagnostic>,
}

/// An AST pattern cannot be passed to the matrix until its syntax has been
/// validated and named fields have been laid out in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternConversionError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Constructor {
    Bool(bool),
    Int(i64),
    Str(String),
    Variant { name: String, arity: usize },
    Tuple(usize),
    Struct { name: String, arity: usize },
}

impl Constructor {
    fn arity(&self) -> usize {
        match self {
            Self::Bool(_) | Self::Int(_) | Self::Str(_) => 0,
            Self::Variant { arity, .. } | Self::Tuple(arity) | Self::Struct { arity, .. } => *arity,
        }
    }
}

/// Run Maranget's P × Q usefulness matrix algorithm on one match expression.
pub fn check_match_exhaustiveness(
    arms: &[PatternArm],
    shape: &TypeShape,
    match_span: Span,
) -> ExhaustivenessReport {
    if arms.len() > MAX_MATCH_ARMS {
        return limit_report(
            match_span,
            format!(
                "match has {} arms; the limit is {MAX_MATCH_ARMS}",
                arms.len()
            ),
        );
    }
    if let Err(message) = validate_shape(shape) {
        return invalid_shape_report(match_span, message);
    }
    let mut matrix = Vec::new();
    let mut unreachable_arms = Vec::new();
    let mut diagnostics = Vec::new();

    for (index, arm) in arms.iter().enumerate() {
        if pattern_depth(&arm.pattern) > MAX_PATTERN_DEPTH {
            diagnostics.push(limit_diagnostic(
                arm.span,
                format!("pattern nesting exceeds the limit of {MAX_PATTERN_DEPTH}"),
            ));
            continue;
        }
        let query = vec![arm.pattern.clone()];
        if useful(&matrix, &query, std::slice::from_ref(shape)).is_none() {
            unreachable_arms.push(index);
            diagnostics.push(unreachable_diagnostic(index, arm.span));
        }
        if !arm.has_guard {
            matrix.push(query);
        }
    }

    let missing_witness = useful(
        &matrix,
        &[SimplePattern::Wildcard],
        std::slice::from_ref(shape),
    )
    .and_then(|mut values| values.pop());
    if let Some(witness) = &missing_witness {
        diagnostics.push(non_exhaustive_diagnostic(witness, match_span));
    }
    ExhaustivenessReport {
        unreachable_arms,
        missing_witness,
        diagnostics,
    }
}

/// Convert an AST pattern into the canonical, positional matrix form.
/// Unsupported surface forms are rejected explicitly rather than being
/// mistaken for redundant arms.
pub fn pattern_from_ast(
    pattern: &Pattern,
    shape: &TypeShape,
) -> Result<SimplePattern, PatternConversionError> {
    match (&pattern.kind, shape) {
        (PatternKind::Wildcard, _) => Ok(SimplePattern::Wildcard),
        (PatternKind::Identifier { name, .. }, TypeShape::EnumWithPayload { variants }) => variants
            .iter()
            .find(|variant| variant.name == name.name && variant.fields.is_empty())
            .map(|variant| SimplePattern::Variant(variant.name.clone()))
            .ok_or_else(|| conversion_error(pattern.span, "identifier bindings are only valid as wildcard patterns here; use a declared unit variant or `_`")),
        (PatternKind::Identifier { .. }, _) => Ok(SimplePattern::Wildcard),
        (PatternKind::Literal(expr), TypeShape::Bool) => match expr.kind {
            ExprKind::BoolLiteral(value) => Ok(SimplePattern::Bool(value)),
            _ => Err(conversion_error(
                pattern.span,
                "expected a boolean literal pattern",
            )),
        },
        (PatternKind::Literal(expr), TypeShape::Int) => match expr.kind {
            ExprKind::IntLiteral(value) => Ok(SimplePattern::Int(value)),
            _ => Err(conversion_error(
                pattern.span,
                "expected an integer literal pattern",
            )),
        },
        (PatternKind::Literal(expr), TypeShape::Str) => match &expr.kind {
            ExprKind::StringLiteral(value) => Ok(SimplePattern::Str(value.clone())),
            _ => Err(conversion_error(
                pattern.span,
                "expected a string literal pattern",
            )),
        },
        (PatternKind::Tuple(patterns), TypeShape::Tuple(fields))
            if patterns.len() == fields.len() =>
        {
            let mut converted = Vec::with_capacity(patterns.len());
            for (field_pattern, field_shape) in patterns.iter().zip(fields) {
                converted.push(pattern_from_ast(field_pattern, field_shape)?);
            }
            Ok(SimplePattern::Tuple(converted))
        }
        (
            PatternKind::Struct { path, fields, rest },
            TypeShape::StructNamed {
                name,
                fields: layout,
            },
        ) => {
            let found_name = path.segments.last().map(|segment| segment.name.as_str());
            if found_name != Some(name.as_str()) {
                return Err(conversion_error(
                    pattern.span,
                    "struct pattern does not match the scrutinee type",
                ));
            }
            let mut converted = Vec::with_capacity(layout.len());
            for (field_name, field_shape) in layout {
                let field = fields.iter().find(|field| field.name.name == *field_name);
                match field {
                    Some(field) => match &field.pattern {
                        Some(value) => converted.push(pattern_from_ast(value, field_shape)?),
                        None => converted.push(SimplePattern::Wildcard),
                    },
                    None if *rest => converted.push(SimplePattern::Wildcard),
                    None => {
                        return Err(conversion_error(
                            pattern.span,
                            format!(
                                "struct pattern is missing field `{field_name}`; add `..` to ignore it"
                            ),
                        ));
                    }
                }
            }
            for field in fields {
                if !layout.iter().any(|(name, _)| name == &field.name.name) {
                    return Err(conversion_error(
                        field.span,
                        format!("unknown field `{}` in struct pattern", field.name.name),
                    ));
                }
            }
            Ok(SimplePattern::Constructor {
                name: name.clone(),
                fields: converted,
            })
        }
        (PatternKind::Variant { path, fields }, TypeShape::EnumWithPayload { variants }) => {
            let Some(name) = path.segments.last().map(|segment| segment.name.clone()) else {
                return Err(conversion_error(
                    pattern.span,
                    "variant pattern has an empty path",
                ));
            };
            let Some(variant) = variants.iter().find(|variant| variant.name == name) else {
                return Err(conversion_error(
                    pattern.span,
                    format!("unknown variant `{name}`"),
                ));
            };
            if fields.len() != variant.fields.len() {
                return Err(conversion_error(
                    pattern.span,
                    format!(
                        "variant `{name}` expects {} field(s), found {}",
                        variant.fields.len(),
                        fields.len()
                    ),
                ));
            }
            if fields.is_empty() {
                Ok(SimplePattern::Variant(name))
            } else {
                let mut converted = Vec::with_capacity(fields.len());
                for (field_pattern, field_shape) in fields.iter().zip(&variant.fields) {
                    converted.push(pattern_from_ast(field_pattern, field_shape)?);
                }
                Ok(SimplePattern::Constructor {
                    name,
                    fields: converted,
                })
            }
        }
        (PatternKind::Binding { pattern: inner, .. }, _) => pattern_from_ast(inner, shape),
        (PatternKind::Typed { pattern: inner, .. }, _) => pattern_from_ast(inner, shape),
        (
            PatternKind::Or(_)
            | PatternKind::Range { .. }
            | PatternKind::Array(_)
            | PatternKind::Rest,
            _,
        ) => Err(conversion_error(
            pattern.span,
            "this pattern form is not yet representable in exhaustiveness checking",
        )),
        _ => Err(conversion_error(
            pattern.span,
            "pattern is incompatible with the scrutinee type",
        )),
    }
}

/// Compatibility entry point for existing, unguarded callers.
pub fn check_exhaustiveness(
    patterns: &[SimplePattern],
    shape: &TypeShape,
    span: Span,
) -> Vec<ExhaustivenessError> {
    let arms: Vec<PatternArm> = patterns
        .iter()
        .cloned()
        .map(|pattern| PatternArm {
            pattern,
            has_guard: false,
            span,
        })
        .collect();
    check_match_exhaustiveness(&arms, shape, span)
        .diagnostics
        .into_iter()
        .map(|diagnostic| ExhaustivenessError {
            message: diagnostic.message,
            span,
        })
        .collect()
}

/// `useful(P, q)`: returns a witness for `q` outside matrix `P`.
fn useful(
    matrix: &[Vec<SimplePattern>],
    query: &[SimplePattern],
    types: &[TypeShape],
) -> Option<Vec<SimplePattern>> {
    if query.is_empty() {
        return if matrix.is_empty() {
            Some(Vec::new())
        } else {
            None
        };
    }
    let (shape, tail_types) = types.split_first()?;
    let (head, tail_query) = query.split_first()?;

    if let Some(constructor) = constructor_for_pattern(head, shape) {
        let mut q = specialize_pattern(head, &constructor, shape)?;
        q.extend_from_slice(tail_query);
        let mut specialized_types = constructor_field_types(&constructor, shape)?;
        specialized_types.extend_from_slice(tail_types);
        let witness = useful(
            &specialize_matrix(matrix, &constructor, shape),
            &q,
            &specialized_types,
        )?;
        return rebuild_witness(constructor, witness);
    }
    if !matches!(head, SimplePattern::Wildcard) {
        // An ill-shaped pattern matches no inhabitant of this column type.
        return None;
    }

    let constructors = constructors_for_type(shape);
    if constructors.is_empty() {
        let mut witness = useful(&default_matrix(matrix), tail_query, tail_types)?;
        witness.insert(0, SimplePattern::Wildcard);
        return Some(witness);
    }
    for constructor in constructors {
        let mut q = wildcard_fields(constructor.arity());
        q.extend_from_slice(tail_query);
        let mut specialized_types = constructor_field_types(&constructor, shape)?;
        specialized_types.extend_from_slice(tail_types);
        if let Some(witness) = useful(
            &specialize_matrix(matrix, &constructor, shape),
            &q,
            &specialized_types,
        ) {
            return rebuild_witness(constructor, witness);
        }
    }
    None
}

fn constructors_for_type(shape: &TypeShape) -> Vec<Constructor> {
    match shape {
        TypeShape::Bool => vec![Constructor::Bool(false), Constructor::Bool(true)],
        TypeShape::Enum { variants } => variants
            .iter()
            .map(|name| Constructor::Variant {
                name: name.clone(),
                arity: 0,
            })
            .collect(),
        TypeShape::EnumWithPayload { variants } => variants
            .iter()
            .map(|variant| Constructor::Variant {
                name: variant.name.clone(),
                arity: variant.fields.len(),
            })
            .collect(),
        TypeShape::Tuple(fields) => vec![Constructor::Tuple(fields.len())],
        TypeShape::Struct { name, fields } => vec![Constructor::Struct {
            name: name.clone(),
            arity: fields.len(),
        }],
        TypeShape::StructNamed { name, fields } => vec![Constructor::Struct {
            name: name.clone(),
            arity: fields.len(),
        }],
        TypeShape::Int | TypeShape::Str | TypeShape::Other => Vec::new(),
    }
}

fn constructor_for_pattern(pattern: &SimplePattern, shape: &TypeShape) -> Option<Constructor> {
    match (pattern, shape) {
        (SimplePattern::Bool(value), TypeShape::Bool) => Some(Constructor::Bool(*value)),
        (SimplePattern::Int(value), TypeShape::Int) => Some(Constructor::Int(*value)),
        (SimplePattern::Str(value), TypeShape::Str) => Some(Constructor::Str(value.clone())),
        (SimplePattern::Variant(name), TypeShape::Enum { variants })
            if variants.iter().any(|variant| variant == name) =>
        {
            Some(Constructor::Variant {
                name: name.clone(),
                arity: 0,
            })
        }
        (SimplePattern::Variant(name), TypeShape::EnumWithPayload { variants }) => variants
            .iter()
            .find(|variant| variant.name == *name && variant.fields.is_empty())
            .map(|variant| Constructor::Variant {
                name: variant.name.clone(),
                arity: 0,
            }),
        (SimplePattern::Constructor { name, fields }, TypeShape::EnumWithPayload { variants }) => {
            variants
                .iter()
                .find(|variant| variant.name == *name && variant.fields.len() == fields.len())
                .map(|variant| Constructor::Variant {
                    name: variant.name.clone(),
                    arity: variant.fields.len(),
                })
        }
        (SimplePattern::Tuple(fields), TypeShape::Tuple(types)) if fields.len() == types.len() => {
            Some(Constructor::Tuple(fields.len()))
        }
        (
            SimplePattern::Constructor { name, fields },
            TypeShape::Struct {
                name: type_name,
                fields: types,
            },
        ) if name == type_name && fields.len() == types.len() => Some(Constructor::Struct {
            name: name.clone(),
            arity: fields.len(),
        }),
        (
            SimplePattern::Constructor { name, fields },
            TypeShape::StructNamed {
                name: type_name,
                fields: types,
            },
        ) if name == type_name && fields.len() == types.len() => Some(Constructor::Struct {
            name: name.clone(),
            arity: fields.len(),
        }),
        _ => None,
    }
}

fn constructor_field_types(constructor: &Constructor, shape: &TypeShape) -> Option<Vec<TypeShape>> {
    match (constructor, shape) {
        (Constructor::Bool(_) | Constructor::Int(_) | Constructor::Str(_), _) => Some(Vec::new()),
        (Constructor::Tuple(arity), TypeShape::Tuple(fields)) if *arity == fields.len() => {
            Some(fields.clone())
        }
        (
            Constructor::Struct { name, arity },
            TypeShape::Struct {
                name: type_name,
                fields,
            },
        ) if name == type_name && *arity == fields.len() => Some(fields.clone()),
        (
            Constructor::Struct { name, arity },
            TypeShape::StructNamed {
                name: type_name,
                fields,
            },
        ) if name == type_name && *arity == fields.len() => {
            Some(fields.iter().map(|(_, ty)| ty.clone()).collect())
        }
        (Constructor::Variant { name, arity }, TypeShape::Enum { variants })
            if *arity == 0 && variants.iter().any(|variant| variant == name) =>
        {
            Some(Vec::new())
        }
        (Constructor::Variant { name, arity }, TypeShape::EnumWithPayload { variants }) => variants
            .iter()
            .find(|variant| variant.name == *name && variant.fields.len() == *arity)
            .map(|variant| variant.fields.clone()),
        _ => None,
    }
}

fn specialize_matrix(
    matrix: &[Vec<SimplePattern>],
    constructor: &Constructor,
    shape: &TypeShape,
) -> Vec<Vec<SimplePattern>> {
    matrix
        .iter()
        .filter_map(|row| {
            let (head, tail) = row.split_first()?;
            let mut specialized = specialize_pattern(head, constructor, shape)?;
            specialized.extend_from_slice(tail);
            Some(specialized)
        })
        .collect()
}

fn specialize_pattern(
    pattern: &SimplePattern,
    constructor: &Constructor,
    shape: &TypeShape,
) -> Option<Vec<SimplePattern>> {
    if matches!(pattern, SimplePattern::Wildcard) {
        return Some(wildcard_fields(constructor.arity()));
    }
    if constructor_for_pattern(pattern, shape).as_ref() != Some(constructor) {
        return None;
    }
    match pattern {
        SimplePattern::Tuple(fields) | SimplePattern::Constructor { fields, .. } => {
            Some(fields.clone())
        }
        SimplePattern::Bool(_)
        | SimplePattern::Int(_)
        | SimplePattern::Str(_)
        | SimplePattern::Variant(_) => Some(Vec::new()),
        SimplePattern::Wildcard => Some(wildcard_fields(constructor.arity())),
    }
}

fn default_matrix(matrix: &[Vec<SimplePattern>]) -> Vec<Vec<SimplePattern>> {
    matrix
        .iter()
        .filter_map(|row| {
            let (head, tail) = row.split_first()?;
            if matches!(head, SimplePattern::Wildcard) {
                Some(tail.to_vec())
            } else {
                None
            }
        })
        .collect()
}

fn wildcard_fields(arity: usize) -> Vec<SimplePattern> {
    std::iter::repeat_n(SimplePattern::Wildcard, arity).collect()
}

fn conversion_error(span: Span, message: impl Into<String>) -> PatternConversionError {
    PatternConversionError {
        message: message.into(),
        span,
    }
}

fn pattern_depth(pattern: &SimplePattern) -> usize {
    match pattern {
        SimplePattern::Tuple(fields) | SimplePattern::Constructor { fields, .. } => {
            1 + fields
                .iter()
                .map(pattern_depth)
                .max()
                .map_or(0, |depth| depth)
        }
        _ => 1,
    }
}

fn validate_shape(shape: &TypeShape) -> Result<(), String> {
    match shape {
        TypeShape::Enum { variants } => {
            validate_names(variants.iter().map(String::as_str), "enum variant")
        }
        TypeShape::EnumWithPayload { variants } => {
            validate_names(
                variants.iter().map(|variant| variant.name.as_str()),
                "enum variant",
            )?;
            for variant in variants {
                for field in &variant.fields {
                    validate_shape(field)?;
                }
            }
            Ok(())
        }
        TypeShape::Tuple(fields) | TypeShape::Struct { fields, .. } => {
            for field in fields {
                validate_shape(field)?;
            }
            Ok(())
        }
        TypeShape::StructNamed { fields, .. } => {
            validate_names(fields.iter().map(|(name, _)| name.as_str()), "struct field")?;
            for (_, field) in fields {
                validate_shape(field)?;
            }
            Ok(())
        }
        TypeShape::Bool | TypeShape::Int | TypeShape::Str | TypeShape::Other => Ok(()),
    }
}

fn validate_names<'a>(names: impl Iterator<Item = &'a str>, kind: &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for name in names {
        if name.is_empty() {
            return Err(format!("{kind} names must not be empty"));
        }
        if !seen.insert(name) {
            return Err(format!("duplicate {kind} `{name}`"));
        }
    }
    Ok(())
}

fn invalid_shape_report(span: Span, message: String) -> ExhaustivenessReport {
    ExhaustivenessReport {
        unreachable_arms: Vec::new(),
        missing_witness: None,
        diagnostics: vec![
            Diagnostic::error("E0005", format!("invalid pattern type shape: {message}"))
                .with_label(Label::primary(
                    span,
                    "the type definition cannot be checked for exhaustiveness",
                )),
        ],
    }
}

fn limit_report(span: Span, message: String) -> ExhaustivenessReport {
    ExhaustivenessReport {
        unreachable_arms: Vec::new(),
        missing_witness: None,
        diagnostics: vec![limit_diagnostic(span, message)],
    }
}

fn limit_diagnostic(span: Span, message: String) -> Diagnostic {
    Diagnostic::error(
        "E0006",
        format!("exhaustiveness analysis limit exceeded: {message}"),
    )
    .with_label(Label::primary(
        span,
        "analysis stopped before matrix expansion became excessive",
    ))
    .with_proof(NyayaProof::new(
        "the match exceeds a resource safety limit",
        "usefulness matrix expansion is bounded to preserve compiler availability",
        Some("split the match into smaller expressions"),
        "semantic analysis must terminate within configured resource limits",
    ))
}

fn rebuild_witness(
    constructor: Constructor,
    values: Vec<SimplePattern>,
) -> Option<Vec<SimplePattern>> {
    let arity = constructor.arity();
    if values.len() < arity {
        return None;
    }
    let mut values = values.into_iter();
    let fields: Vec<SimplePattern> = values.by_ref().take(arity).collect();
    let head = match constructor {
        Constructor::Bool(value) => SimplePattern::Bool(value),
        Constructor::Int(value) => SimplePattern::Int(value),
        Constructor::Str(value) => SimplePattern::Str(value),
        Constructor::Variant { name, arity: 0 } => SimplePattern::Variant(name),
        Constructor::Variant { name, .. } | Constructor::Struct { name, .. } => {
            SimplePattern::Constructor { name, fields }
        }
        Constructor::Tuple(_) => SimplePattern::Tuple(fields),
    };
    let mut rebuilt = vec![head];
    rebuilt.extend(values);
    Some(rebuilt)
}

fn non_exhaustive_diagnostic(witness: &SimplePattern, span: Span) -> Diagnostic {
    let value = render_pattern(witness);
    Diagnostic::error(
        "E0004",
        format!("non-exhaustive match: missing pattern `{value}`"),
    )
    .with_label(Label::primary(
        span,
        "this match does not cover every possible value",
    ))
    .with_help(format!("add an arm for `{value}` or add a final `_` arm"))
    .with_proof(NyayaProof::new(
        format!("the usefulness matrix produced uncovered value `{value}`"),
        "no preceding unguarded arm matches that value",
        Some(format!("add `{value} => ...` or `_ => ...`")),
        "a match expression must cover every value of its scrutinee type",
    ))
}

fn unreachable_diagnostic(index: usize, span: Span) -> Diagnostic {
    let arm = index + 1;
    Diagnostic::warning("W0004", format!("unreachable match arm {arm}"))
        .with_label(Label::primary(
            span,
            "this pattern is covered by earlier unguarded arms",
        ))
        .with_proof(NyayaProof::new(
            format!("arm {arm} has no useful witness"),
            "the preceding unguarded matrix covers every value matched by this arm",
            Some("remove this arm or make an earlier arm more specific"),
            "each reachable match arm must match a value not matched by earlier unguarded arms",
        ))
}

pub fn render_pattern(pattern: &SimplePattern) -> String {
    match pattern {
        SimplePattern::Wildcard => "_".into(),
        SimplePattern::Bool(value) => value.to_string(),
        SimplePattern::Int(value) => value.to_string(),
        SimplePattern::Str(value) => format!("{value:?}"),
        SimplePattern::Variant(name) => name.clone(),
        SimplePattern::Tuple(fields) => format!(
            "({})",
            fields
                .iter()
                .map(render_pattern)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        SimplePattern::Constructor { name, fields } => format!(
            "{}({})",
            name,
            fields
                .iter()
                .map(render_pattern)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span() -> Span {
        Span::dummy()
    }
    fn arm(pattern: SimplePattern) -> PatternArm {
        PatternArm {
            pattern,
            has_guard: false,
            span: span(),
        }
    }
    fn option_of(inner: TypeShape) -> TypeShape {
        TypeShape::EnumWithPayload {
            variants: vec![
                VariantShape {
                    name: "None".into(),
                    fields: Vec::new(),
                },
                VariantShape {
                    name: "Some".into(),
                    fields: vec![inner],
                },
            ],
        }
    }

    #[test]
    fn bool_is_exhaustive() {
        let report = check_match_exhaustiveness(
            &[
                arm(SimplePattern::Bool(true)),
                arm(SimplePattern::Bool(false)),
            ],
            &TypeShape::Bool,
            span(),
        );
        assert!(report.missing_witness.is_none());
        assert!(report.unreachable_arms.is_empty());
    }

    #[test]
    fn nested_tuple_produces_a_concrete_witness() {
        let shape = TypeShape::Tuple(vec![TypeShape::Bool, option_of(TypeShape::Bool)]);
        let report = check_match_exhaustiveness(
            &[
                arm(SimplePattern::Tuple(vec![
                    SimplePattern::Bool(false),
                    SimplePattern::Wildcard,
                ])),
                arm(SimplePattern::Tuple(vec![
                    SimplePattern::Bool(true),
                    SimplePattern::Constructor {
                        name: "Some".into(),
                        fields: vec![SimplePattern::Wildcard],
                    },
                ])),
            ],
            &shape,
            span(),
        );
        assert_eq!(
            report.missing_witness.as_ref().map(render_pattern),
            Some("(true, None)".into())
        );
    }

    #[test]
    fn deep_enum_payload_requires_nested_case() {
        let shape = option_of(TypeShape::Tuple(vec![
            TypeShape::Bool,
            option_of(TypeShape::Bool),
        ]));
        let report = check_match_exhaustiveness(
            &[
                arm(SimplePattern::Variant("None".into())),
                arm(SimplePattern::Constructor {
                    name: "Some".into(),
                    fields: vec![SimplePattern::Tuple(vec![
                        SimplePattern::Wildcard,
                        SimplePattern::Constructor {
                            name: "Some".into(),
                            fields: vec![SimplePattern::Wildcard],
                        },
                    ])],
                }),
            ],
            &shape,
            span(),
        );
        assert_eq!(
            report.missing_witness.as_ref().map(render_pattern),
            Some("Some((false, None))".into())
        );
    }

    #[test]
    fn guard_does_not_cover_values() {
        let guarded = PatternArm {
            pattern: SimplePattern::Wildcard,
            has_guard: true,
            span: span(),
        };
        let report = check_match_exhaustiveness(&[guarded], &TypeShape::Bool, span());
        assert_eq!(report.missing_witness, Some(SimplePattern::Bool(false)));
    }

    #[test]
    fn guarded_arm_does_not_make_following_arm_redundant() {
        let guarded = PatternArm {
            pattern: SimplePattern::Bool(true),
            has_guard: true,
            span: span(),
        };
        let report = check_match_exhaustiveness(
            &[
                guarded,
                arm(SimplePattern::Bool(true)),
                arm(SimplePattern::Bool(false)),
            ],
            &TypeShape::Bool,
            span(),
        );
        assert!(report.unreachable_arms.is_empty());
        assert!(report.missing_witness.is_none());
    }

    #[test]
    fn redundant_nested_arm_is_detected() {
        let report = check_match_exhaustiveness(
            &[
                arm(SimplePattern::Constructor {
                    name: "Some".into(),
                    fields: vec![SimplePattern::Wildcard],
                }),
                arm(SimplePattern::Constructor {
                    name: "Some".into(),
                    fields: vec![SimplePattern::Bool(true)],
                }),
                arm(SimplePattern::Variant("None".into())),
            ],
            &option_of(TypeShape::Bool),
            span(),
        );
        assert_eq!(report.unreachable_arms, vec![1]);
        assert!(report.missing_witness.is_none());
    }

    #[test]
    fn struct_product_space_is_checked() {
        let shape = TypeShape::Struct {
            name: "Point".into(),
            fields: vec![TypeShape::Bool, TypeShape::Bool],
        };
        let report = check_match_exhaustiveness(
            &[
                arm(SimplePattern::Constructor {
                    name: "Point".into(),
                    fields: vec![SimplePattern::Bool(false), SimplePattern::Wildcard],
                }),
                arm(SimplePattern::Constructor {
                    name: "Point".into(),
                    fields: vec![SimplePattern::Bool(true), SimplePattern::Bool(false)],
                }),
            ],
            &shape,
            span(),
        );
        assert_eq!(
            report.missing_witness.as_ref().map(render_pattern),
            Some("Point(true, true)".into())
        );
    }

    #[test]
    fn diagnostics_include_nyaya_proof() {
        let report = check_match_exhaustiveness(&[], &TypeShape::Bool, span());
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].proof.is_some());
    }

    #[test]
    fn compatibility_api_reports_witness() {
        let errors = check_exhaustiveness(&[SimplePattern::Bool(true)], &TypeShape::Bool, span());
        assert!(errors.iter().any(|error| error.message.contains("false")));
    }

    #[test]
    fn duplicate_variants_are_rejected_before_matrix_construction() {
        let shape = TypeShape::Enum {
            variants: vec!["Same".into(), "Same".into()],
        };
        let report = check_match_exhaustiveness(&[], &shape, span());
        assert!(
            report.diagnostics[0]
                .message
                .contains("duplicate enum variant")
        );
    }

    #[test]
    fn named_struct_adapter_rejects_unknown_fields() {
        let pattern = Pattern {
            id: agam_ast::NodeId(0),
            span: span(),
            kind: PatternKind::Struct {
                path: agam_ast::Path {
                    segments: vec![agam_ast::Ident::new("Point", span())],
                    span: span(),
                },
                fields: vec![agam_ast::pattern::FieldPattern {
                    name: agam_ast::Ident::new("z", span()),
                    pattern: None,
                    span: span(),
                }],
                rest: true,
            },
        };
        let shape = TypeShape::StructNamed {
            name: "Point".into(),
            fields: vec![("x".into(), TypeShape::Int)],
        };
        let result = pattern_from_ast(&pattern, &shape);
        assert!(matches!(result, Err(error) if error.message.contains("unknown field `z`")));
    }

    #[test]
    fn analysis_limit_is_reported_without_expanding_the_matrix() {
        let arms = vec![arm(SimplePattern::Wildcard); MAX_MATCH_ARMS + 1];
        let report = check_match_exhaustiveness(&arms, &TypeShape::Bool, span());
        assert!(report.diagnostics[0].message.contains("limit exceeded"));
    }
}
