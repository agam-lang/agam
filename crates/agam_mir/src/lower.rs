//! HIR → MIR lowering pass.
//!
//! Transforms the high-level HIR into basic-block-based MIR with SSA values.

use agam_hir::nodes::*;
use agam_sema::symbol::TypeId;
use agam_sema::types::TypeStore;

use crate::ir::*;

/// The MIR lowering context.
pub struct MirLowering {
    next_value: u32,
    next_block: u32,
    blocks: Vec<BasicBlock>,
    current_instrs: Vec<Instruction>,
    types: TypeStore,
}

impl MirLowering {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            blocks: Vec::new(),
            current_instrs: Vec::new(),
            types: TypeStore::new(),
        }
    }

    fn fresh_value(&mut self) -> ValueId {
        let id = ValueId(self.next_value);
        self.next_value += 1;
        id
    }

    fn fresh_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    fn emit(&mut self, ty: TypeId, op: Op) -> ValueId {
        let result = self.fresh_value();
        self.current_instrs.push(Instruction { result, ty, op });
        result
    }

    fn finish_block(&mut self, id: BlockId, terminator: Terminator) {
        let instrs = std::mem::take(&mut self.current_instrs);
        self.blocks.push(BasicBlock {
            id,
            instructions: instrs,
            terminator,
        });
    }

    /// Lower an entire HIR module into MIR.
    pub fn lower_module(&mut self, hir: &HirModule) -> MirModule {
        let functions = hir.functions.iter().map(|f| self.lower_function(f)).collect();
        MirModule { functions }
    }

    fn lower_function(&mut self, func: &HirFunction) -> MirFunction {
        self.blocks.clear();
        self.current_instrs.clear();

        let entry = self.fresh_block();

        // Emit parameter allocas
        let params: Vec<MirParam> = func.params.iter().map(|p| {
            let v = self.fresh_value();
            MirParam { value: v, ty: p.ty }
        }).collect();

        // Lower body
        let result = self.lower_block(&func.body);

        // Finish entry block with return
        match result {
            Some(val) => self.finish_block(entry, Terminator::Return(val)),
            None => {
                let unit = self.emit(self.types.unit(), Op::Unit);
                self.finish_block(entry, Terminator::Return(unit));
            }
        }

        MirFunction {
            name: func.name.clone(),
            params,
            return_ty: func.return_ty,
            blocks: std::mem::take(&mut self.blocks),
            entry,
        }
    }

    fn lower_block(&mut self, block: &HirBlock) -> Option<ValueId> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        block.expr.as_ref().map(|e| self.lower_expr(e))
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let { name, ty, value, .. } => {
                self.emit(*ty, Op::Alloca { name: name.clone(), ty: *ty });
                if let Some(val_expr) = value {
                    let val = self.lower_expr(val_expr);
                    self.emit(*ty, Op::StoreLocal { name: name.clone(), value: val });
                }
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            HirStmt::Return(val) => {
                // Return is handled by block terminator — emit value for now.
                if let Some(v) = val {
                    self.lower_expr(v);
                }
            }
            HirStmt::While { condition, body } => {
                // Simplified: just lower the condition and body inline.
                let _cond = self.lower_expr(condition);
                self.lower_block(body);
            }
            HirStmt::If { condition, then_branch, else_branch } => {
                let _cond = self.lower_expr(condition);
                self.lower_block(then_branch);
                if let Some(eb) = else_branch {
                    self.lower_block(eb);
                }
            }
            _ => {}
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> ValueId {
        let ty = expr.ty;
        match &expr.kind {
            HirExprKind::IntLit(v) => self.emit(ty, Op::ConstInt(*v)),
            HirExprKind::FloatLit(v) => self.emit(ty, Op::ConstFloat(*v)),
            HirExprKind::BoolLit(v) => self.emit(ty, Op::ConstBool(*v)),
            HirExprKind::StringLit(v) => self.emit(ty, Op::ConstString(v.clone())),

            HirExprKind::Var(name) => self.emit(ty, Op::LoadLocal(name.clone())),

            HirExprKind::Binary { op, left, right } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                self.emit(ty, Op::BinOp { op: lower_binop(*op), left: l, right: r })
            }
            HirExprKind::Unary { op, operand } => {
                let v = self.lower_expr(operand);
                self.emit(ty, Op::UnOp { op: lower_unop(*op), operand: v })
            }

            HirExprKind::Call { callee, args } => {
                let callee_name = match &callee.kind {
                    HirExprKind::Var(name) => name.clone(),
                    _ => "__indirect_call".into(),
                };
                let arg_vals: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a)).collect();
                self.emit(ty, Op::Call { callee: callee_name, args: arg_vals })
            }

            HirExprKind::MethodCall { object, method, args } => {
                let obj_val = self.lower_expr(object);
                let mut all_args = vec![obj_val];
                all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                self.emit(ty, Op::Call { callee: method.clone(), args: all_args })
            }

            HirExprKind::FieldAccess { object, field } => {
                let obj = self.lower_expr(object);
                self.emit(ty, Op::GetField { object: obj, field: field.clone() })
            }
            HirExprKind::Index { object, index } => {
                let obj = self.lower_expr(object);
                let idx = self.lower_expr(index);
                self.emit(ty, Op::GetIndex { object: obj, index: idx })
            }

            HirExprKind::Assign { target, value } => {
                let val = self.lower_expr(value);
                if let HirExprKind::Var(name) = &target.kind {
                    self.emit(ty, Op::StoreLocal { name: name.clone(), value: val })
                } else {
                    val
                }
            }

            HirExprKind::Array(elems) | HirExprKind::Tuple(elems) => {
                for e in elems { self.lower_expr(e); }
                self.emit(ty, Op::Unit)
            }

            HirExprKind::Block(block) => {
                self.lower_block(block).unwrap_or_else(|| self.emit(ty, Op::Unit))
            }

            HirExprKind::Cast { expr: inner, target_ty } => {
                let v = self.lower_expr(inner);
                self.emit(*target_ty, Op::Cast { value: v, target_ty: *target_ty })
            }
        }
    }
}

fn lower_binop(op: HirBinOp) -> MirBinOp {
    match op {
        HirBinOp::Add => MirBinOp::Add,
        HirBinOp::Sub => MirBinOp::Sub,
        HirBinOp::Mul => MirBinOp::Mul,
        HirBinOp::Div => MirBinOp::Div,
        HirBinOp::Mod => MirBinOp::Mod,
        HirBinOp::Eq => MirBinOp::Eq,
        HirBinOp::NotEq => MirBinOp::NotEq,
        HirBinOp::Lt => MirBinOp::Lt,
        HirBinOp::LtEq => MirBinOp::LtEq,
        HirBinOp::Gt => MirBinOp::Gt,
        HirBinOp::GtEq => MirBinOp::GtEq,
        HirBinOp::And => MirBinOp::And,
        HirBinOp::Or => MirBinOp::Or,
        HirBinOp::BitAnd => MirBinOp::BitAnd,
        HirBinOp::BitOr => MirBinOp::BitOr,
        HirBinOp::BitXor => MirBinOp::BitXor,
        HirBinOp::Shl => MirBinOp::Shl,
        HirBinOp::Shr => MirBinOp::Shr,
        HirBinOp::Pow => MirBinOp::Mul, // Simplified for now
    }
}

fn lower_unop(op: HirUnaryOp) -> MirUnOp {
    match op {
        HirUnaryOp::Neg => MirUnOp::Neg,
        HirUnaryOp::Not => MirUnOp::Not,
        HirUnaryOp::BitNot => MirUnOp::BitNot,
        _ => MirUnOp::Not, // Ref/Deref handled at higher level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_errors::span::SourceId;

    fn lower_to_mir(source: &str) -> MirModule {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof { break; }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);

        let mut mir_lower = MirLowering::new();
        mir_lower.lower_module(&hir)
    }

    #[test]
    fn test_mir_simple_function() {
        let mir = lower_to_mir("fn main(): return 42");
        assert_eq!(mir.functions.len(), 1);
        assert_eq!(mir.functions[0].name, "main");
        assert!(!mir.functions[0].blocks.is_empty());
    }

    #[test]
    fn test_mir_has_const_int() {
        let mir = lower_to_mir("fn main(): return 42");
        let f = &mir.functions[0];
        let has_int = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| matches!(&i.op, Op::ConstInt(42)))
        });
        assert!(has_int, "expected ConstInt(42) in MIR");
    }

    #[test]
    fn test_mir_binary_op() {
        let mir = lower_to_mir("fn main(): let x = 1 + 2");
        let f = &mir.functions[0];
        let has_add = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| matches!(&i.op, Op::BinOp { op: MirBinOp::Add, .. }))
        });
        assert!(has_add, "expected BinOp::Add in MIR");
    }

    #[test]
    fn test_mir_function_call() {
        let mir = lower_to_mir("fn main(): print(42)");
        let f = &mir.functions[0];
        let has_call = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| matches!(&i.op, Op::Call { callee, .. } if callee == "print"))
        });
        assert!(has_call, "expected Call to 'print' in MIR");
    }

    #[test]
    fn test_mir_return_terminates() {
        let mir = lower_to_mir("fn main(): return 42");
        let f = &mir.functions[0];
        let entry = &f.blocks[0];
        assert!(matches!(&entry.terminator, Terminator::Return(_)));
    }
}
