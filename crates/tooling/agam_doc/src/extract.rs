//! Documentation extraction from AST and doc comments.

use agam_ast::Module;
use agam_ast::decl::{
    DeclKind, EffectDecl, EnumDecl, FunctionDecl, GenericParam, StructDecl, TraitDecl, TraitItem,
    VariantFields, Visibility,
};
use agam_ast::pattern::{Pattern, PatternKind};
use agam_ast::types::{TypeExpr, TypeExprKind};

use crate::model::{
    DocEffect, DocEnum, DocField, DocFunction, DocItem, DocModule, DocPackage, DocParam, DocStruct,
    DocTrait, DocTypeAlias, DocVariant, Doctest, SearchEntry,
};

/// Extract a `DocPackage` from a parsed AST `Module` and metadata.
pub fn extract_package(
    package_name: &str,
    version: &str,
    description: Option<&str>,
    module: &Module,
) -> DocPackage {
    let root_module = extract_module("crate", &[], module);
    let mut search_index = Vec::new();

    collect_search_entries(&root_module, &mut search_index);

    DocPackage {
        name: package_name.to_string(),
        version: version.to_string(),
        description: description.map(|d| d.to_string()),
        root_module,
        search_index,
    }
}

/// Extract documentation for a module from AST declarations.
pub fn extract_module(name: &str, path: &[String], module: &Module) -> DocModule {
    let mut items = Vec::new();
    let mut submodules = Vec::new();

    for decl in &module.declarations {
        match &decl.kind {
            DeclKind::Function(f) => {
                if !f.name.name.starts_with("__") {
                    items.push(DocItem::Function(extract_function(f, &decl.doc_comments)));
                }
            }
            DeclKind::Struct(s) => {
                items.push(DocItem::Struct(extract_struct(s, &decl.doc_comments)));
            }
            DeclKind::Enum(e) => {
                items.push(DocItem::Enum(extract_enum(e, &decl.doc_comments)));
            }
            DeclKind::Trait(t) => {
                items.push(DocItem::Trait(extract_trait(t, &decl.doc_comments)));
            }
            DeclKind::Effect(ef) => {
                items.push(DocItem::Effect(extract_effect(ef, &decl.doc_comments)));
            }
            DeclKind::TypeAlias {
                name,
                generics,
                ty,
                visibility,
            } => {
                items.push(DocItem::TypeAlias(extract_type_alias(
                    &name.name,
                    generics,
                    ty,
                    *visibility,
                    &decl.doc_comments,
                )));
            }
            DeclKind::Module(m) => {
                let mut sub_path = path.to_vec();
                sub_path.push(m.name.name.clone());
                let sub_mod = DocModule {
                    name: m.name.name.clone(),
                    path: sub_path,
                    docs: decl.doc_comments.clone(),
                    items: Vec::new(),
                    submodules: Vec::new(),
                };
                submodules.push(sub_mod);
            }
            _ => {}
        }
    }

    DocModule {
        name: name.to_string(),
        path: path.to_vec(),
        docs: module.doc_comments.clone(),
        items,
        submodules,
    }
}

/// Extract function documentation and doctests.
fn extract_function(f: &FunctionDecl, docs: &[String]) -> DocFunction {
    let doctests = extract_doctests(&f.name.name, docs);
    let mut params = Vec::new();

    for p in &f.params {
        let param_name = format_pattern(&p.pattern);
        let ty_str = format_type_expr(&p.ty);
        params.push(DocParam {
            name: param_name,
            ty: ty_str,
        });
    }

    let return_type = f.return_type.as_ref().map(format_type_expr);
    let generics: Vec<String> = f.generics.iter().map(format_generic_param).collect();

    let sig_params = params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    let sig_ret = return_type
        .as_ref()
        .map(|r| format!(" -> {}", r))
        .unwrap_or_default();

    let sig_gen = if f.generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };

    let async_prefix = if f.is_async { "async " } else { "" };
    let signature = format!(
        "{async_prefix}fn {}{sig_gen}({sig_params}){sig_ret}",
        f.name.name
    );

    DocFunction {
        name: f.name.name.clone(),
        docs: docs.to_vec(),
        signature,
        generics,
        params,
        return_type,
        is_async: f.is_async,
        visibility: format_visibility(f.visibility),
        doctests,
    }
}

/// Extract struct documentation.
fn extract_struct(s: &StructDecl, docs: &[String]) -> DocStruct {
    let mut fields = Vec::new();
    for field in &s.fields {
        fields.push(DocField {
            name: field.name.name.clone(),
            ty: format_type_expr(&field.ty),
            docs: Vec::new(),
        });
    }

    let generics: Vec<String> = s.generics.iter().map(format_generic_param).collect();

    DocStruct {
        name: s.name.name.clone(),
        docs: docs.to_vec(),
        generics,
        fields,
        visibility: format_visibility(s.visibility),
    }
}

/// Extract enum documentation.
fn extract_enum(e: &EnumDecl, docs: &[String]) -> DocEnum {
    let mut variants = Vec::new();
    for v in &e.variants {
        let payload = match &v.fields {
            VariantFields::Unit => None,
            VariantFields::Tuple(tys) => Some(format!(
                "({})",
                tys.iter()
                    .map(format_type_expr)
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            VariantFields::Struct(fields) => Some(format!(
                " {{ {} }}",
                fields
                    .iter()
                    .map(|f| format!("{}: {}", f.name.name, format_type_expr(&f.ty)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        };

        variants.push(DocVariant {
            name: v.name.name.clone(),
            payload,
            docs: Vec::new(),
        });
    }

    let generics: Vec<String> = e.generics.iter().map(format_generic_param).collect();

    DocEnum {
        name: e.name.name.clone(),
        docs: docs.to_vec(),
        generics,
        variants,
        visibility: format_visibility(e.visibility),
    }
}

/// Extract trait documentation.
fn extract_trait(t: &TraitDecl, docs: &[String]) -> DocTrait {
    let mut methods = Vec::new();
    for item in &t.items {
        if let TraitItem::Method(m) = item {
            methods.push(extract_function(m, &[]));
        }
    }

    let generics: Vec<String> = t.generics.iter().map(format_generic_param).collect();

    DocTrait {
        name: t.name.name.clone(),
        docs: docs.to_vec(),
        generics,
        methods,
        visibility: format_visibility(t.visibility),
    }
}

/// Extract effect documentation.
fn extract_effect(ef: &EffectDecl, docs: &[String]) -> DocEffect {
    let mut operations = Vec::new();
    for op in &ef.operations {
        let op_params = op
            .params
            .iter()
            .map(|(name, ty)| DocParam {
                name: name.name.clone(),
                ty: format_type_expr(ty),
            })
            .collect::<Vec<_>>();

        let sig_params = op_params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty))
            .collect::<Vec<_>>()
            .join(", ");

        let ret = op.return_type.as_ref().map(format_type_expr);
        let sig_ret = ret.as_ref().map(|r| format!(" -> {r}")).unwrap_or_default();
        let signature = format!("fn {}({sig_params}){sig_ret}", op.name.name);

        operations.push(DocFunction {
            name: op.name.name.clone(),
            docs: Vec::new(),
            signature,
            generics: Vec::new(),
            params: op_params,
            return_type: ret,
            is_async: false,
            visibility: "pub".to_string(),
            doctests: Vec::new(),
        });
    }

    DocEffect {
        name: ef.name.name.clone(),
        docs: docs.to_vec(),
        operations,
        visibility: format_visibility(ef.visibility),
    }
}

/// Extract type alias documentation.
fn extract_type_alias(
    name: &str,
    generics: &[GenericParam],
    ty: &TypeExpr,
    vis: Visibility,
    docs: &[String],
) -> DocTypeAlias {
    let generics_vec: Vec<String> = generics.iter().map(format_generic_param).collect();
    DocTypeAlias {
        name: name.to_string(),
        docs: docs.to_vec(),
        generics: generics_vec,
        target_type: format_type_expr(ty),
        visibility: format_visibility(vis),
    }
}

/// Extract markdown code blocks for doctesting.
pub fn extract_doctests(item_name: &str, docs: &[String]) -> Vec<Doctest> {
    let mut doctests = Vec::new();
    let mut in_block = false;
    let mut current_code = Vec::new();
    let mut start_line = 0;
    let mut should_panic = false;
    let mut ignore = false;

    for (line_idx, line) in docs.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_block {
                // Closing fence
                let code = current_code.join("\n");
                doctests.push(Doctest {
                    item_name: item_name.to_string(),
                    code,
                    line: start_line,
                    should_panic,
                    ignore,
                });
                current_code.clear();
                in_block = false;
            } else {
                // Opening fence
                let tag = trimmed.trim_start_matches('`').trim();
                let is_agam = tag.is_empty() || tag.starts_with("agam") || tag.starts_with("rust");
                if is_agam {
                    in_block = true;
                    start_line = line_idx + 1;
                    should_panic = tag.contains("should_panic");
                    ignore = tag.contains("ignore") || tag.contains("no_run");
                }
            }
        } else if in_block {
            current_code.push(line.as_str());
        }
    }

    doctests
}

/// Helper to format type expressions into readable strings.
pub fn format_type_expr(ty: &TypeExpr) -> String {
    match &ty.kind {
        TypeExprKind::Named(p) => p
            .segments
            .iter()
            .map(|id| id.name.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        TypeExprKind::Generic { base, args } => {
            let base_str = base
                .segments
                .iter()
                .map(|id| id.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            let arg_strs = args
                .iter()
                .map(format_type_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{base_str}<{arg_strs}>")
        }
        TypeExprKind::Array { element, size: _ } => {
            format!("[{}]", format_type_expr(element))
        }
        TypeExprKind::Slice(element) => {
            format!("[{}]", format_type_expr(element))
        }
        TypeExprKind::Tuple(types) => {
            let inner = types
                .iter()
                .map(format_type_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({inner})")
        }
        TypeExprKind::Function {
            params,
            return_type,
        } => {
            let p_strs = params
                .iter()
                .map(format_type_expr)
                .collect::<Vec<_>>()
                .join(", ");
            let ret_str = format_type_expr(return_type);
            format!("fn({p_strs}) -> {ret_str}")
        }
        TypeExprKind::Reference { mutable, inner } => {
            let m = if *mutable { "mut " } else { "" };
            format!("&{m}{}", format_type_expr(inner))
        }
        TypeExprKind::Pointer { mutable, inner } => {
            let m = if *mutable { "mut " } else { "" };
            format!("*{m}{}", format_type_expr(inner))
        }
        TypeExprKind::Optional(inner) => {
            format!("{}?", format_type_expr(inner))
        }
        TypeExprKind::Result { ok, err } => {
            format!(
                "Result<{}, {}>",
                format_type_expr(ok),
                format_type_expr(err)
            )
        }
        TypeExprKind::Refined { base, .. } => format_type_expr(base),
        TypeExprKind::DynTrait(inner) => format!("dyn {}", format_type_expr(inner)),
        TypeExprKind::Any => "Any".to_string(),
        TypeExprKind::SelfType => "Self".to_string(),
        TypeExprKind::Inferred => "_".to_string(),
        TypeExprKind::Never => "!".to_string(),
        TypeExprKind::Dynamic => "dyn Any".to_string(),
    }
}

pub fn format_pattern(p: &Pattern) -> String {
    match &p.kind {
        PatternKind::Identifier { name, mutable } => {
            if *mutable {
                format!("mut {}", name.name)
            } else {
                name.name.clone()
            }
        }
        PatternKind::Wildcard => "_".to_string(),
        _ => "param".to_string(),
    }
}

fn format_generic_param(g: &GenericParam) -> String {
    if g.bounds.is_empty() {
        g.name.name.clone()
    } else {
        let bounds = g
            .bounds
            .iter()
            .map(format_type_expr)
            .collect::<Vec<_>>()
            .join(" + ");
        format!("{}: {bounds}", g.name.name)
    }
}

fn format_visibility(vis: Visibility) -> String {
    match vis {
        Visibility::Public => "pub".to_string(),
        Visibility::Private => "".to_string(),
    }
}

fn collect_search_entries(module: &DocModule, entries: &mut Vec<SearchEntry>) {
    let mod_path = if module.path.is_empty() {
        module.name.clone()
    } else {
        module.path.join("::")
    };

    for item in &module.items {
        let summary = item.docs().first().cloned().unwrap_or_default();
        entries.push(SearchEntry {
            name: item.name().to_string(),
            kind: item.item_type().to_string(),
            path: format!("{mod_path}::{}", item.name()),
            summary,
        });
    }

    for sub in &module.submodules {
        collect_search_entries(sub, entries);
    }
}
