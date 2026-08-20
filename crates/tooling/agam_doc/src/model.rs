//! Documentation data models.

use serde::{Deserialize, Serialize};

/// A complete documented package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPackage {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub root_module: DocModule,
    pub search_index: Vec<SearchEntry>,
}

/// A documented module containing submodules and items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocModule {
    pub name: String,
    pub path: Vec<String>,
    pub docs: Vec<String>,
    pub items: Vec<DocItem>,
    pub submodules: Vec<DocModule>,
}

/// A documented top-level item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DocItem {
    Function(DocFunction),
    Struct(DocStruct),
    Enum(DocEnum),
    Trait(DocTrait),
    Effect(DocEffect),
    TypeAlias(DocTypeAlias),
}

impl DocItem {
    pub fn name(&self) -> &str {
        match self {
            DocItem::Function(f) => &f.name,
            DocItem::Struct(s) => &s.name,
            DocItem::Enum(e) => &e.name,
            DocItem::Trait(t) => &t.name,
            DocItem::Effect(ef) => &ef.name,
            DocItem::TypeAlias(t) => &t.name,
        }
    }

    pub fn item_type(&self) -> &'static str {
        match self {
            DocItem::Function(_) => "function",
            DocItem::Struct(_) => "struct",
            DocItem::Enum(_) => "enum",
            DocItem::Trait(_) => "trait",
            DocItem::Effect(_) => "effect",
            DocItem::TypeAlias(_) => "type",
        }
    }

    pub fn docs(&self) -> &[String] {
        match self {
            DocItem::Function(f) => &f.docs,
            DocItem::Struct(s) => &s.docs,
            DocItem::Enum(e) => &e.docs,
            DocItem::Trait(t) => &t.docs,
            DocItem::Effect(ef) => &ef.docs,
            DocItem::TypeAlias(t) => &t.docs,
        }
    }
}

/// Function documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFunction {
    pub name: String,
    pub docs: Vec<String>,
    pub signature: String,
    pub generics: Vec<String>,
    pub params: Vec<DocParam>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub visibility: String,
    pub doctests: Vec<Doctest>,
}

/// Parameter documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocParam {
    pub name: String,
    pub ty: String,
}

/// Struct documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStruct {
    pub name: String,
    pub docs: Vec<String>,
    pub generics: Vec<String>,
    pub fields: Vec<DocField>,
    pub visibility: String,
}

/// Struct field documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocField {
    pub name: String,
    pub ty: String,
    pub docs: Vec<String>,
}

/// Enum documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEnum {
    pub name: String,
    pub docs: Vec<String>,
    pub generics: Vec<String>,
    pub variants: Vec<DocVariant>,
    pub visibility: String,
}

/// Enum variant documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocVariant {
    pub name: String,
    pub payload: Option<String>,
    pub docs: Vec<String>,
}

/// Trait documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTrait {
    pub name: String,
    pub docs: Vec<String>,
    pub generics: Vec<String>,
    pub methods: Vec<DocFunction>,
    pub visibility: String,
}

/// Effect documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEffect {
    pub name: String,
    pub docs: Vec<String>,
    pub operations: Vec<DocFunction>,
    pub visibility: String,
}

/// Type alias documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTypeAlias {
    pub name: String,
    pub docs: Vec<String>,
    pub generics: Vec<String>,
    pub target_type: String,
    pub visibility: String,
}

/// Extracted doctest from doc comment code fences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Doctest {
    pub item_name: String,
    pub code: String,
    pub line: usize,
    pub should_panic: bool,
    pub ignore: bool,
}

/// Search entry for client-side fuzzy search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntry {
    pub name: String,
    pub kind: String,
    pub path: String,
    pub summary: String,
}
