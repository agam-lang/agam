//! AST pretty-printer for debugging and display.

use crate::Module;
use crate::decl::*;
use crate::expr::*;
use crate::stmt::*;

/// Pretty-print an AST module as indented text.
pub fn pretty_print(module: &Module) -> String {
    let mut printer = PrettyPrinter::new();
    printer.print_module(module);
    printer.output
}

struct PrettyPrinter {
    output: String,
    indent: usize,
}

impl PrettyPrinter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn line(&mut self, text: &str) {
        let indent = "  ".repeat(self.indent);
        self.output.push_str(&format!("{}{}\n", indent, text));
    }

    fn print_module(&mut self, module: &Module) {
        self.line("Module:");
        self.indent += 1;
        for decl in &module.declarations {
            self.print_decl(decl);
        }
        self.indent -= 1;
    }

    fn print_decl(&mut self, decl: &Decl) {
        match &decl.kind {
            DeclKind::Function(f) => {
                let async_str = if f.is_async { "async " } else { "" };
                self.line(&format!("{}fn {}(...):", async_str, f.name.name));
                if let Some(body) = &f.body {
                    self.indent += 1;
                    self.print_block(body);
                    self.indent -= 1;
                }
            }
            DeclKind::Struct(s) => {
                self.line(&format!("struct {}:", s.name.name));
                self.indent += 1;
                for field in &s.fields {
                    self.line(&format!("{}: <type>", field.name.name));
                }
                self.indent -= 1;
            }
            DeclKind::Enum(e) => {
                self.line(&format!("enum {}:", e.name.name));
                self.indent += 1;
                for variant in &e.variants {
                    self.line(&format!("{}", variant.name.name));
                }
                self.indent -= 1;
            }
            DeclKind::Trait(t) => {
                self.line(&format!("trait {}:", t.name.name));
            }
            DeclKind::Impl(i) => {
                self.line("impl:");
                self.indent += 1;
                for item in &i.items {
                    self.print_decl(item);
                }
                self.indent -= 1;
            }
            DeclKind::Use(u) => {
                let path: Vec<&str> = u.path.segments.iter().map(|s| s.name.as_str()).collect();
                self.line(&format!("use {}", path.join("::")));
            }
            DeclKind::Module(m) => {
                self.line(&format!("mod {}:", m.name.name));
            }
            DeclKind::TypeAlias { name, .. } => {
                self.line(&format!("type {} = ...", name.name));
            }
            DeclKind::Effect(e) => {
                self.line(&format!("effect {}:", e.name.name));
                self.indent += 1;
                for op in &e.operations {
                    self.line(&format!("fn {}(...)", op.name.name));
                }
                self.indent -= 1;
            }
            DeclKind::Handler(h) => {
                self.line(&format!(
                    "handle {} for {}:",
                    h.name.name, h.effect_name.name
                ));
            }
        }
    }

    fn print_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.print_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.print_expr(expr);
        }
    }

    fn print_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                let val_str = if value.is_some() { " = ..." } else { "" };
                self.line(&format!("let {:?}{}", pattern.kind, val_str));
            }
            StmtKind::Return(expr) => {
                let val = if expr.is_some() { " ..." } else { "" };
                self.line(&format!("return{}", val));
            }
            StmtKind::Expression(expr) => {
                self.print_expr(expr);
            }
            StmtKind::While { body, .. } => {
                self.line("while ...:");
                self.indent += 1;
                self.print_block(body);
                self.indent -= 1;
            }
            StmtKind::For { body, .. } => {
                self.line("for ... in ...:");
                self.indent += 1;
                self.print_block(body);
                self.indent -= 1;
            }
            StmtKind::If { then_branch, .. } => {
                self.line("if ...:");
                self.indent += 1;
                self.print_block(then_branch);
                self.indent -= 1;
            }
            _ => {
                self.line(&format!("{:?}", stmt.kind));
            }
        }
    }

    fn print_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::IntLiteral(n) => self.line(&format!("Int({})", n)),
            ExprKind::FloatLiteral(n) => self.line(&format!("Float({})", n)),
            ExprKind::StringLiteral(s) => self.line(&format!("String(\"{}\")", s)),
            ExprKind::BoolLiteral(b) => self.line(&format!("Bool({})", b)),
            ExprKind::Identifier(id) => self.line(&format!("Ident({})", id.name)),
            ExprKind::Binary { op, .. } => self.line(&format!("BinOp({:?})", op)),
            ExprKind::Call { .. } => self.line("Call(...)"),
            ExprKind::FieldAccess { field, .. } => self.line(&format!("Field(.{})", field.name)),
            ExprKind::MethodCall { method, .. } => {
                self.line(&format!("Method(.{}())", method.name))
            }
            _ => self.line(&format!("{:?}", std::mem::discriminant(&expr.kind))),
        }
    }
}
