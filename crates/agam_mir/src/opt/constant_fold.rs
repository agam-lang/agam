//! Constant folding and local constant propagation for MIR.

use std::collections::HashMap;

use crate::ir::{MirBinOp, MirModule, MirUnOp, Op, Terminator, ValueId};

#[derive(Clone, Debug, PartialEq)]
enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

impl Constant {
    fn to_op(&self) -> Op {
        match self {
            Constant::Int(value) => Op::ConstInt(*value),
            Constant::Float(value) => Op::ConstFloat(*value),
            Constant::Bool(value) => Op::ConstBool(*value),
            Constant::String(value) => Op::ConstString(value.clone()),
            Constant::Unit => Op::Unit,
        }
    }
}

pub fn run(module: &mut MirModule) -> bool {
    let mut changed = false;

    for function in &mut module.functions {
        for block in &mut function.blocks {
            let mut value_consts: HashMap<ValueId, Constant> = HashMap::new();
            let mut local_consts: HashMap<String, Constant> = HashMap::new();

            for instr in &mut block.instructions {
                match &mut instr.op {
                    Op::ConstInt(value) => {
                        value_consts.insert(instr.result, Constant::Int(*value));
                    }
                    Op::ConstFloat(value) => {
                        value_consts.insert(instr.result, Constant::Float(*value));
                    }
                    Op::ConstBool(value) => {
                        value_consts.insert(instr.result, Constant::Bool(*value));
                    }
                    Op::ConstString(value) => {
                        value_consts.insert(instr.result, Constant::String(value.clone()));
                    }
                    Op::Unit => {
                        value_consts.insert(instr.result, Constant::Unit);
                    }
                    Op::Copy(value) => {
                        if let Some(constant) = value_consts.get(value).cloned() {
                            instr.op = constant.to_op();
                            value_consts.insert(instr.result, constant);
                            changed = true;
                        } else {
                            value_consts.remove(&instr.result);
                        }
                    }
                    Op::LoadLocal(name) => {
                        if let Some(constant) = local_consts.get(name).cloned() {
                            instr.op = constant.to_op();
                            value_consts.insert(instr.result, constant);
                            changed = true;
                        } else {
                            value_consts.remove(&instr.result);
                        }
                    }
                    Op::StoreLocal { name, value } => {
                        if let Some(constant) = value_consts.get(value).cloned() {
                            local_consts.insert(name.clone(), constant);
                        } else {
                            local_consts.remove(name);
                        }
                        value_consts.remove(&instr.result);
                    }
                    Op::BinOp { op, left, right } => {
                        let folded = value_consts
                            .get(left)
                            .zip(value_consts.get(right))
                            .and_then(|(left, right)| fold_binop(*op, left, right));

                        if let Some(constant) = folded {
                            instr.op = constant.to_op();
                            value_consts.insert(instr.result, constant);
                            changed = true;
                        } else {
                            value_consts.remove(&instr.result);
                        }
                    }
                    Op::UnOp { op, operand } => {
                        let folded = value_consts
                            .get(operand)
                            .and_then(|operand| fold_unop(*op, operand));

                        if let Some(constant) = folded {
                            instr.op = constant.to_op();
                            value_consts.insert(instr.result, constant);
                            changed = true;
                        } else {
                            value_consts.remove(&instr.result);
                        }
                    }
                    Op::Cast { value, .. } => {
                        if let Some(constant) = value_consts.get(value).cloned() {
                            instr.op = constant.to_op();
                            value_consts.insert(instr.result, constant);
                            changed = true;
                        } else {
                            value_consts.remove(&instr.result);
                        }
                    }
                    Op::Call { .. }
                    | Op::Alloca { .. }
                    | Op::GetField { .. }
                    | Op::GetIndex { .. }
                    | Op::Phi(_) => {
                        value_consts.remove(&instr.result);
                    }
                }
            }

            if let Terminator::Branch {
                condition,
                then_block,
                else_block,
            } = &block.terminator
            {
                if let Some(Constant::Bool(condition)) = value_consts.get(condition) {
                    block.terminator = Terminator::Jump(if *condition {
                        *then_block
                    } else {
                        *else_block
                    });
                    changed = true;
                }
            }
        }
    }

    changed
}

fn fold_binop(op: MirBinOp, left: &Constant, right: &Constant) -> Option<Constant> {
    match (op, left, right) {
        (MirBinOp::Add, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Int(left.wrapping_add(*right)))
        }
        (MirBinOp::Sub, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Int(left.wrapping_sub(*right)))
        }
        (MirBinOp::Mul, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Int(left.wrapping_mul(*right)))
        }
        (MirBinOp::Div, Constant::Int(left), Constant::Int(right)) if *right != 0 => {
            Some(Constant::Int(left.wrapping_div(*right)))
        }
        (MirBinOp::Mod, Constant::Int(left), Constant::Int(right)) if *right != 0 => {
            Some(Constant::Int(left.wrapping_rem(*right)))
        }
        (MirBinOp::Eq, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left == right))
        }
        (MirBinOp::NotEq, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left != right))
        }
        (MirBinOp::Lt, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left < right))
        }
        (MirBinOp::LtEq, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left <= right))
        }
        (MirBinOp::Gt, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left > right))
        }
        (MirBinOp::GtEq, Constant::Int(left), Constant::Int(right)) => {
            Some(Constant::Bool(left >= right))
        }
        (MirBinOp::And, Constant::Bool(left), Constant::Bool(right)) => {
            Some(Constant::Bool(*left && *right))
        }
        (MirBinOp::Or, Constant::Bool(left), Constant::Bool(right)) => {
            Some(Constant::Bool(*left || *right))
        }
        (MirBinOp::Eq, Constant::Bool(left), Constant::Bool(right)) => {
            Some(Constant::Bool(left == right))
        }
        (MirBinOp::NotEq, Constant::Bool(left), Constant::Bool(right)) => {
            Some(Constant::Bool(left != right))
        }
        (MirBinOp::Add, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Float(left + right))
        }
        (MirBinOp::Sub, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Float(left - right))
        }
        (MirBinOp::Mul, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Float(left * right))
        }
        (MirBinOp::Div, Constant::Float(left), Constant::Float(right)) if *right != 0.0 => {
            Some(Constant::Float(left / right))
        }
        (MirBinOp::Eq, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left == right))
        }
        (MirBinOp::NotEq, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left != right))
        }
        (MirBinOp::Lt, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left < right))
        }
        (MirBinOp::LtEq, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left <= right))
        }
        (MirBinOp::Gt, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left > right))
        }
        (MirBinOp::GtEq, Constant::Float(left), Constant::Float(right)) => {
            Some(Constant::Bool(left >= right))
        }
        _ => None,
    }
}

fn fold_unop(op: MirUnOp, operand: &Constant) -> Option<Constant> {
    match (op, operand) {
        (MirUnOp::Neg, Constant::Int(value)) => Some(Constant::Int(value.wrapping_neg())),
        (MirUnOp::Neg, Constant::Float(value)) => Some(Constant::Float(-value)),
        (MirUnOp::Not, Constant::Bool(value)) => Some(Constant::Bool(!value)),
        (MirUnOp::BitNot, Constant::Int(value)) => Some(Constant::Int(!value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    use crate::{ir::{MirBinOp, Op, Terminator}, lower::MirLowering};

    use super::run;

    fn optimize_source(source: &str) -> crate::ir::MirModule {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }

        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);

        let mut mir_lower = MirLowering::new();
        let mut mir = mir_lower.lower_module(&hir);
        run(&mut mir);
        mir
    }

    #[test]
    fn folds_literal_arithmetic() {
        let mir = optimize_source("fn main(): let x = 5 * 10");
        let instructions = &mir.functions[0].blocks[0].instructions;

        assert!(instructions.iter().any(|instr| matches!(instr.op, Op::ConstInt(50))));
        assert!(!instructions.iter().any(|instr| matches!(instr.op, Op::BinOp { op: MirBinOp::Mul, .. })));
    }

    #[test]
    fn propagates_constants_through_locals() {
        let mir = optimize_source("fn main() { let x = 5 * 10; let y = x + 2; }");
        let instructions = &mir.functions[0].blocks[0].instructions;

        assert!(instructions.iter().any(|instr| matches!(instr.op, Op::ConstInt(52))));
        assert!(!instructions.iter().any(|instr| matches!(instr.op, Op::LoadLocal(_))));
    }

    #[test]
    fn folds_constant_branches_into_jumps() {
        let mir = optimize_source("fn main() { if 1 < 2 { return 7; } else { return 9; } }");
        let entry = &mir.functions[0].blocks[0];

        assert!(matches!(entry.terminator, Terminator::Jump(_)));
    }
}
