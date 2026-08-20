//! High-aesthetic HTML documentation generator.

use std::fs;
use std::path::Path;

use crate::model::{DocEnum, DocFunction, DocItem, DocModule, DocPackage, DocStruct, DocTrait};

/// Generate full HTML documentation tree in target directory.
pub fn generate_html(package: &DocPackage, out_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(out_dir)?;

    // Write CSS stylesheet
    fs::write(out_dir.join("style.css"), THEME_CSS)?;

    // Write JavaScript search & interactions
    fs::write(out_dir.join("script.js"), THEME_JS)?;

    // Write search index
    let search_json =
        serde_json::to_string(&package.search_index).unwrap_or_else(|_| "[]".to_string());
    fs::write(
        out_dir.join("search-index.js"),
        format!("window.AGAM_SEARCH_INDEX = {search_json};"),
    )?;

    // Render root index.html
    let index_html = render_package_index(package);
    fs::write(out_dir.join("index.html"), index_html)?;

    // Render individual item pages
    render_module_pages(package, &package.root_module, out_dir)?;

    Ok(())
}

fn render_package_index(pkg: &DocPackage) -> String {
    let mut item_sections = String::new();

    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut effects = Vec::new();
    let mut type_aliases = Vec::new();

    for item in &pkg.root_module.items {
        match item {
            DocItem::Function(f) => functions.push(f),
            DocItem::Struct(s) => structs.push(s),
            DocItem::Enum(e) => enums.push(e),
            DocItem::Trait(t) => traits.push(t),
            DocItem::Effect(ef) => effects.push(ef),
            DocItem::TypeAlias(ta) => type_aliases.push(ta),
        }
    }

    if !structs.is_empty() {
        item_sections.push_str("<h2>Structs</h2>\n<div class=\"item-grid\">\n");
        for s in structs {
            let doc_summary = s.docs.first().cloned().unwrap_or_default();
            item_sections.push_str(&format!(
                "<a class=\"item-card\" href=\"struct.{}.html\"><div class=\"item-title\"><span class=\"badge struct\">struct</span><strong>{}</strong></div><p class=\"item-desc\">{}</p></a>\n",
                s.name, s.name, escape_html(&doc_summary)
            ));
        }
        item_sections.push_str("</div>\n");
    }

    if !enums.is_empty() {
        item_sections.push_str("<h2>Enums</h2>\n<div class=\"item-grid\">\n");
        for e in enums {
            let doc_summary = e.docs.first().cloned().unwrap_or_default();
            item_sections.push_str(&format!(
                "<a class=\"item-card\" href=\"enum.{}.html\"><div class=\"item-title\"><span class=\"badge enum\">enum</span><strong>{}</strong></div><p class=\"item-desc\">{}</p></a>\n",
                e.name, e.name, escape_html(&doc_summary)
            ));
        }
        item_sections.push_str("</div>\n");
    }

    if !traits.is_empty() {
        item_sections.push_str("<h2>Traits</h2>\n<div class=\"item-grid\">\n");
        for t in traits {
            let doc_summary = t.docs.first().cloned().unwrap_or_default();
            item_sections.push_str(&format!(
                "<a class=\"item-card\" href=\"trait.{}.html\"><div class=\"item-title\"><span class=\"badge trait\">trait</span><strong>{}</strong></div><p class=\"item-desc\">{}</p></a>\n",
                t.name, t.name, escape_html(&doc_summary)
            ));
        }
        item_sections.push_str("</div>\n");
    }

    if !functions.is_empty() {
        item_sections.push_str("<h2>Functions</h2>\n<div class=\"item-grid\">\n");
        for f in functions {
            let doc_summary = f.docs.first().cloned().unwrap_or_default();
            item_sections.push_str(&format!(
                "<a class=\"item-card\" href=\"fn.{}.html\"><div class=\"item-title\"><span class=\"badge fn\">fn</span><strong>{}</strong></div><p class=\"item-desc\">{}</p></a>\n",
                f.name, f.name, escape_html(&doc_summary)
            ));
        }
        item_sections.push_str("</div>\n");
    }

    let module_docs = pkg.root_module.docs.join("\n\n");
    let rendered_docs = render_markdown(&module_docs);

    wrap_page(
        &pkg.name,
        &pkg.version,
        &format!("Crate {}", pkg.name),
        &format!(
            "<div class=\"crate-header\"><h1>Crate <span class=\"highlight\">{}</span> <span class=\"version-tag\">v{}</span></h1><p class=\"crate-desc\">{}</p></div>\n<div class=\"module-docs\">{}</div>\n<div class=\"crate-contents\">{}</div>",
            pkg.name,
            pkg.version,
            pkg.description.as_deref().unwrap_or(""),
            rendered_docs,
            item_sections
        ),
        &render_sidebar(pkg, ""),
    )
}

fn render_module_pages(
    pkg: &DocPackage,
    module: &DocModule,
    out_dir: &Path,
) -> std::io::Result<()> {
    for item in &module.items {
        match item {
            DocItem::Function(f) => {
                let html = render_function_page(pkg, f);
                fs::write(out_dir.join(format!("fn.{}.html", f.name)), html)?;
            }
            DocItem::Struct(s) => {
                let html = render_struct_page(pkg, s);
                fs::write(out_dir.join(format!("struct.{}.html", s.name)), html)?;
            }
            DocItem::Enum(e) => {
                let html = render_enum_page(pkg, e);
                fs::write(out_dir.join(format!("enum.{}.html", e.name)), html)?;
            }
            DocItem::Trait(t) => {
                let html = render_trait_page(pkg, t);
                fs::write(out_dir.join(format!("trait.{}.html", t.name)), html)?;
            }
            _ => {}
        }
    }

    for sub in &module.submodules {
        render_module_pages(pkg, sub, out_dir)?;
    }

    Ok(())
}

fn render_function_page(pkg: &DocPackage, f: &DocFunction) -> String {
    let docs = render_markdown(&f.docs.join("\n\n"));
    let mut params_section = String::new();
    if !f.params.is_empty() {
        params_section.push_str("<h3>Parameters</h3>\n<ul class=\"param-list\">\n");
        for p in &f.params {
            params_section.push_str(&format!(
                "<li><code>{}</code>: <code>{}</code></li>\n",
                p.name, p.ty
            ));
        }
        params_section.push_str("</ul>\n");
    }

    let ret_section = if let Some(r) = &f.return_type {
        format!("<h3>Returns</h3>\n<p><code>{}</code></p>\n", r)
    } else {
        String::new()
    };

    let content = format!(
        "<div class=\"item-header\"><span class=\"badge fn\">function</span><h1>{}</h1></div>\n<pre class=\"code-block\"><code>{}</code></pre>\n<div class=\"item-documentation\">{docs}\n{params_section}\n{ret_section}</div>",
        f.name,
        escape_html(&f.signature)
    );

    wrap_page(
        &pkg.name,
        &pkg.version,
        &format!("fn {} - {}", f.name, pkg.name),
        &content,
        &render_sidebar(pkg, &f.name),
    )
}

fn render_struct_page(pkg: &DocPackage, s: &DocStruct) -> String {
    let docs = render_markdown(&s.docs.join("\n\n"));
    let mut fields_section = String::new();
    if !s.fields.is_empty() {
        fields_section.push_str("<h3>Fields</h3>\n<div class=\"field-table\">\n");
        for field in &s.fields {
            fields_section.push_str(&format!(
                "<div class=\"field-row\"><code>{}</code>: <code>{}</code></div>\n",
                field.name, field.ty
            ));
        }
        fields_section.push_str("</div>\n");
    }

    let content = format!(
        "<div class=\"item-header\"><span class=\"badge struct\">struct</span><h1>{}</h1></div>\n<div class=\"item-documentation\">{docs}\n{fields_section}</div>",
        s.name
    );

    wrap_page(
        &pkg.name,
        &pkg.version,
        &format!("struct {} - {}", s.name, pkg.name),
        &content,
        &render_sidebar(pkg, &s.name),
    )
}

fn render_enum_page(pkg: &DocPackage, e: &DocEnum) -> String {
    let docs = render_markdown(&e.docs.join("\n\n"));
    let mut variants_section = String::new();
    if !e.variants.is_empty() {
        variants_section.push_str("<h3>Variants</h3>\n<div class=\"variant-list\">\n");
        for v in &e.variants {
            let payload = v.payload.as_deref().unwrap_or("");
            variants_section.push_str(&format!(
                "<div class=\"variant-row\"><code>{}{}</code></div>\n",
                v.name, payload
            ));
        }
        variants_section.push_str("</div>\n");
    }

    let content = format!(
        "<div class=\"item-header\"><span class=\"badge enum\">enum</span><h1>{}</h1></div>\n<div class=\"item-documentation\">{docs}\n{variants_section}</div>",
        e.name
    );

    wrap_page(
        &pkg.name,
        &pkg.version,
        &format!("enum {} - {}", e.name, pkg.name),
        &content,
        &render_sidebar(pkg, &e.name),
    )
}

fn render_trait_page(pkg: &DocPackage, t: &DocTrait) -> String {
    let docs = render_markdown(&t.docs.join("\n\n"));
    let mut methods_section = String::new();
    if !t.methods.is_empty() {
        methods_section.push_str("<h3>Required Methods</h3>\n<div class=\"method-list\">\n");
        for m in &t.methods {
            methods_section.push_str(&format!(
                "<div class=\"method-row\"><pre class=\"code-block\"><code>{}</code></pre></div>\n",
                escape_html(&m.signature)
            ));
        }
        methods_section.push_str("</div>\n");
    }

    let content = format!(
        "<div class=\"item-header\"><span class=\"badge trait\">trait</span><h1>{}</h1></div>\n<div class=\"item-documentation\">{docs}\n{methods_section}</div>",
        t.name
    );

    wrap_page(
        &pkg.name,
        &pkg.version,
        &format!("trait {} - {}", t.name, pkg.name),
        &content,
        &render_sidebar(pkg, &t.name),
    )
}

fn render_sidebar(pkg: &DocPackage, current_item: &str) -> String {
    let mut sidebar = format!(
        "<div class=\"sidebar-brand\"><a href=\"index.html\"><h3>{}</h3></a><span class=\"sidebar-version\">v{}</span></div>\n",
        pkg.name, pkg.version
    );

    sidebar.push_str("<div class=\"sidebar-search\"><input type=\"text\" id=\"search-box\" placeholder=\"Search types, functions... (/) \" autocomplete=\"off\" /></div>\n");
    sidebar.push_str("<nav class=\"sidebar-nav\">\n");

    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut fns = Vec::new();

    for item in &pkg.root_module.items {
        match item {
            DocItem::Struct(s) => structs.push(&s.name),
            DocItem::Enum(e) => enums.push(&e.name),
            DocItem::Trait(t) => traits.push(&t.name),
            DocItem::Function(f) => fns.push(&f.name),
            _ => {}
        }
    }

    if !structs.is_empty() {
        sidebar.push_str("<div class=\"sidebar-group\"><h4>Structs</h4><ul>");
        for s in structs {
            let active = if s == current_item {
                " class=\"active\""
            } else {
                ""
            };
            sidebar.push_str(&format!(
                "<li><a href=\"struct.{s}.html\"{active}>{s}</a></li>"
            ));
        }
        sidebar.push_str("</ul></div>");
    }

    if !enums.is_empty() {
        sidebar.push_str("<div class=\"sidebar-group\"><h4>Enums</h4><ul>");
        for e in enums {
            let active = if e == current_item {
                " class=\"active\""
            } else {
                ""
            };
            sidebar.push_str(&format!(
                "<li><a href=\"enum.{e}.html\"{active}>{e}</a></li>"
            ));
        }
        sidebar.push_str("</ul></div>");
    }

    if !traits.is_empty() {
        sidebar.push_str("<div class=\"sidebar-group\"><h4>Traits</h4><ul>");
        for t in traits {
            let active = if t == current_item {
                " class=\"active\""
            } else {
                ""
            };
            sidebar.push_str(&format!(
                "<li><a href=\"trait.{t}.html\"{active}>{t}</a></li>"
            ));
        }
        sidebar.push_str("</ul></div>");
    }

    if !fns.is_empty() {
        sidebar.push_str("<div class=\"sidebar-group\"><h4>Functions</h4><ul>");
        for f in fns {
            let active = if f == current_item {
                " class=\"active\""
            } else {
                ""
            };
            sidebar.push_str(&format!("<li><a href=\"fn.{f}.html\"{active}>{f}</a></li>"));
        }
        sidebar.push_str("</ul></div>");
    }

    sidebar.push_str("</nav>");
    sidebar
}

fn wrap_page(pkg_name: &str, _version: &str, title: &str, content: &str, sidebar: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="layout-container">
        <aside class="sidebar">{sidebar}</aside>
        <main class="main-content">
            <div class="top-nav">
                <a href="index.html" class="nav-crumb">{pkg_name}</a>
            </div>
            <article class="content-body">
                {content}
            </article>
            <div id="search-modal" class="search-modal hidden">
                <div class="search-modal-content">
                    <input type="text" id="modal-search-input" placeholder="Search..." />
                    <ul id="search-results-list"></ul>
                </div>
            </div>
        </main>
    </div>
    <script src="search-index.js"></script>
    <script src="script.js"></script>
</body>
</html>"#
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_markdown(md: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                out.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                out.push_str("<pre class=\"code-block\"><code>");
                in_code_block = true;
            }
        } else if in_code_block {
            out.push_str(&escape_html(line));
            out.push('\n');
        } else if trimmed.starts_with('#') {
            let count = trimmed.chars().take_while(|c| *c == '#').count();
            let heading_text = trimmed.trim_start_matches('#').trim();
            out.push_str(&format!("<h{count}>{heading_text}</h{count}>\n"));
        } else if !trimmed.is_empty() {
            out.push_str(&format!("<p>{}</p>\n", escape_html(line)));
        }
    }

    if in_code_block {
        out.push_str("</code></pre>\n");
    }

    out
}

const THEME_CSS: &str = r#"
:root {
    --bg-primary: #0d1117;
    --bg-secondary: #161b22;
    --bg-card: rgba(22, 27, 34, 0.7);
    --border-color: #30363d;
    --accent-primary: #38bdf8;
    --accent-glow: rgba(56, 189, 248, 0.2);
    --text-primary: #f0f6fc;
    --text-muted: #8b949e;
    --code-bg: #0b0f14;
    --badge-fn: #3b82f6;
    --badge-struct: #10b981;
    --badge-enum: #f59e0b;
    --badge-trait: #8b5cf6;
}

* { box-sizing: border-box; margin: 0; padding: 0; }
body {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    line-height: 1.6;
}

.layout-container { display: flex; min-height: 100vh; }
.sidebar {
    width: 280px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    padding: 1.5rem;
    position: sticky;
    top: 0;
    height: 100vh;
    overflow-y: auto;
}

.sidebar-brand h3 { color: var(--accent-primary); margin-bottom: 0.25rem; }
.sidebar-version { font-size: 0.8rem; color: var(--text-muted); }
.sidebar-search input {
    width: 100%;
    margin-top: 1rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
}

.sidebar-nav { margin-top: 1.5rem; }
.sidebar-group { margin-bottom: 1.25rem; }
.sidebar-group h4 { font-size: 0.85rem; text-transform: uppercase; color: var(--text-muted); margin-bottom: 0.5rem; }
.sidebar-group ul { list-style: none; }
.sidebar-group li a { color: var(--text-primary); text-decoration: none; display: block; padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.9rem; }
.sidebar-group li a:hover, .sidebar-group li a.active { background: var(--accent-glow); color: var(--accent-primary); }

.main-content { flex: 1; padding: 2rem 3rem; max-width: 1000px; }
.top-nav { margin-bottom: 2rem; color: var(--text-muted); }
.top-nav a { color: var(--accent-primary); text-decoration: none; }

.crate-header h1 { font-size: 2.5rem; margin-bottom: 0.5rem; }
.crate-header .highlight { color: var(--accent-primary); }
.version-tag { font-size: 1rem; background: var(--bg-secondary); padding: 0.25rem 0.6rem; border-radius: 9999px; border: 1px solid var(--border-color); vertical-align: middle; }
.crate-desc { color: var(--text-muted); font-size: 1.1rem; margin-bottom: 2rem; }

.item-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 1rem; margin-top: 1rem; margin-bottom: 2rem; }
.item-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    text-decoration: none;
    color: var(--text-primary);
    transition: all 0.2s ease;
}
.item-card:hover { border-color: var(--accent-primary); transform: translateY(-2px); }
.item-title { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; }
.item-desc { color: var(--text-muted); font-size: 0.85rem; }

.badge { font-size: 0.75rem; padding: 0.15rem 0.45rem; border-radius: 4px; text-transform: uppercase; font-weight: bold; color: #fff; }
.badge.fn { background: var(--badge-fn); }
.badge.struct { background: var(--badge-struct); }
.badge.enum { background: var(--badge-enum); }
.badge.trait { background: var(--badge-trait); }

.code-block {
    background: var(--code-bg);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    overflow-x: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.95rem;
    margin: 1rem 0;
}

.item-documentation { margin-top: 1.5rem; }
.item-documentation h3 { margin: 1.5rem 0 0.5rem 0; color: var(--accent-primary); }
.param-list { list-style: square; padding-left: 1.5rem; margin: 0.5rem 0; }
.field-table, .variant-list { margin: 0.5rem 0; }
.field-row, .variant-row { padding: 0.5rem; background: var(--bg-secondary); border-left: 3px solid var(--accent-primary); margin-bottom: 0.25rem; border-radius: 0 4px 4px 0; }

.search-modal { position: fixed; inset: 0; background: rgba(0,0,0,0.8); display: flex; align-items: center; justify-content: center; z-index: 1000; }
.search-modal.hidden { display: none; }
.search-modal-content { background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: 12px; width: 600px; max-width: 90vw; padding: 1.5rem; }
.search-modal-content input { width: 100%; padding: 0.75rem; background: var(--bg-primary); border: 1px solid var(--border-color); border-radius: 6px; color: var(--text-primary); font-size: 1.1rem; }
#search-results-list { list-style: none; margin-top: 1rem; max-height: 350px; overflow-y: auto; }
#search-results-list li a { display: block; padding: 0.5rem; color: var(--text-primary); text-decoration: none; border-radius: 4px; }
#search-results-list li a:hover { background: var(--accent-glow); color: var(--accent-primary); }
"#;

const THEME_JS: &str = r#"
document.addEventListener('DOMContentLoaded', () => {
    const searchBox = document.getElementById('search-box');
    const modal = document.getElementById('search-modal');
    const modalInput = document.getElementById('modal-search-input');
    const resultsList = document.getElementById('search-results-list');

    function openSearch() {
        if (modal) {
            modal.classList.remove('hidden');
            modalInput.focus();
        }
    }

    function closeSearch() {
        if (modal) modal.classList.add('hidden');
    }

    if (searchBox) {
        searchBox.addEventListener('focus', openSearch);
    }

    window.addEventListener('keydown', (e) => {
        if (e.key === '/' && document.activeElement !== searchBox && document.activeElement !== modalInput) {
            e.preventDefault();
            openSearch();
        } else if (e.key === 'Escape') {
            closeSearch();
        }
    });

    if (modal) {
        modal.addEventListener('click', (e) => {
            if (e.target === modal) closeSearch();
        });
    }

    if (modalInput && window.AGAM_SEARCH_INDEX) {
        modalInput.addEventListener('input', (e) => {
            const query = e.target.value.toLowerCase().trim();
            resultsList.innerHTML = '';
            if (!query) return;

            const matches = window.AGAM_SEARCH_INDEX.filter(item => 
                item.name.toLowerCase().includes(query) || item.path.toLowerCase().includes(query)
            );

            matches.slice(0, 10).forEach(item => {
                const li = document.createElement('li');
                const a = document.createElement('a');
                a.href = item.kind === 'function' ? `fn.${item.name}.html` :
                         item.kind === 'struct' ? `struct.${item.name}.html` :
                         item.kind === 'enum' ? `enum.${item.name}.html` :
                         item.kind === 'trait' ? `trait.${item.name}.html` : `index.html`;
                a.innerHTML = `<strong>[${item.kind}]</strong> ${item.path} - <span style="color:#8b949e">${item.summary}</span>`;
                li.appendChild(a);
                resultsList.appendChild(li);
            });
        });
    }
});
"#;
