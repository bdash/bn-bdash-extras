//! Pattern-matchable wrappers around Binary Ninja's `low_level_il` types
//!
//! The medium- and high-level IL representation has a lifted variant that serves this purpose
//! match against, but that does not yet exist for low-level IL.
//!
//! This currently only supports the operations I've had a need to match against. It will need
//! to be expanded as it is used for more things.

use std::convert::Into;

use binaryninja::low_level_il::{
    LowLevelILRegisterKind, LowLevelILSSARegisterKind,
    expression::{ExpressionHandler as _, LowLevelILExpression, LowLevelILExpressionKind},
    instruction::{InstructionHandler as _, LowLevelILInstruction, LowLevelILInstructionKind},
};

mod bn {
    pub use binaryninja::{
        architecture::CoreRegister,
        low_level_il::{
            expression::{ExpressionHandler, ValueExpr},
            function::{FunctionForm, FunctionMutability},
            instruction::InstructionHandler,
            operation::{BinaryOp, Operation},
        },
    };
}

#[derive(Debug)]
pub enum Instruction<'a, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    If(
        Expression<'a, M, F>,
        LowLevelILInstruction<'a, M, F>,
        LowLevelILInstruction<'a, M, F>,
    ),
    SetReg(
        LowLevelILRegisterKind<bn::CoreRegister>,
        Expression<'a, M, F>,
    ),
    SetRegSsa(
        LowLevelILSSARegisterKind<bn::CoreRegister>,
        Expression<'a, M, F>,
    ),
    RegPhi(
        LowLevelILSSARegisterKind<bn::CoreRegister>,
        Vec<LowLevelILSSARegisterKind<bn::CoreRegister>>,
    ),
    Call(Expression<'a, M, F>),
    TailCall(Expression<'a, M, F>),
    CallSsa(Expression<'a, M, F>, Vec<Expression<'a, M, F>>),
    TailCallSsa(Expression<'a, M, F>, Vec<Expression<'a, M, F>>),
    Goto(LowLevelILInstruction<'a, M, F>),
    Jump(Expression<'a, M, F>),
    Unknown(&'a LowLevelILInstruction<'a, M, F>),
}

#[derive(Debug)]
pub struct BinaryExpression<'a, M, F>(pub Expression<'a, M, F>, pub Expression<'a, M, F>)
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm;

#[derive(Debug)]
pub enum Expression<'a, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    Add(Box<BinaryExpression<'a, M, F>>),
    Sub(Box<BinaryExpression<'a, M, F>>),
    And(Box<BinaryExpression<'a, M, F>>),
    Xor(Box<BinaryExpression<'a, M, F>>),
    Lsl(Box<BinaryExpression<'a, M, F>>),
    Lsr(Box<BinaryExpression<'a, M, F>>),
    CmpE(Box<BinaryExpression<'a, M, F>>),
    Reg(LowLevelILRegisterKind<bn::CoreRegister>),
    RegSsa(LowLevelILSSARegisterKind<bn::CoreRegister>),
    Const(u64),
    ConstPtr(u64),
    Unknown(LowLevelILExpression<'a, M, F, bn::ValueExpr>),
}

impl<'a, 'b, M, F> From<&'b LowLevelILInstruction<'a, M, F>> for Instruction<'b, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'a, M, F>: bn::InstructionHandler<'a, M, F>,
    LowLevelILExpression<'a, M, F, bn::ValueExpr>: bn::ExpressionHandler<'a, M, F>,
{
    fn from(instr: &'b LowLevelILInstruction<'a, M, F>) -> Self {
        use LowLevelILExpressionKind as ExpressionKind;
        use LowLevelILInstructionKind as Kind;
        match instr.kind() {
            Kind::If(operation) => Self::If(
                operation.condition().into(),
                operation.true_target(),
                operation.false_target(),
            ),
            Kind::SetReg(operation) => {
                Instruction::SetReg(operation.dest_reg(), operation.source_expr().into())
            }
            Kind::SetRegSsa(operation) => {
                Instruction::SetRegSsa(operation.dest_reg(), operation.source_expr().into())
            }
            Kind::Call(operation) => Instruction::Call(operation.target().into()),
            Kind::TailCall(operation) => Instruction::TailCall(operation.target().into()),
            Kind::CallSsa(operation) => {
                let ExpressionKind::CallParamSsa(params) = operation.param_expr().kind() else {
                    panic!(
                        "Unexpected call parameter expression kind: {:?}",
                        operation.param_expr().kind()
                    );
                };
                Instruction::CallSsa(
                    operation.target().into(),
                    params.param_exprs().into_iter().map(Into::into).collect(),
                )
            }
            Kind::TailCallSsa(operation) => {
                let ExpressionKind::CallParamSsa(params) = operation.param_expr().kind() else {
                    panic!(
                        "Unexpected call parameter expression kind: {:?}",
                        operation.param_expr().kind()
                    );
                };
                Instruction::TailCallSsa(
                    operation.target().into(),
                    params.param_exprs().into_iter().map(Into::into).collect(),
                )
            }
            Kind::RegPhi(operation) => {
                let source_regs = operation.source_regs();
                Instruction::RegPhi(operation.dest_reg(), source_regs)
            }
            Kind::Goto(operation) => Instruction::Goto(operation.target()),
            Kind::Jump(operation) => Instruction::Jump(operation.target().into()),
            _ => Instruction::Unknown(instr),
        }
    }
}

impl<'a, M, F> From<LowLevelILExpression<'a, M, F, bn::ValueExpr>> for Expression<'a, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'a, M, F, bn::ValueExpr>: bn::ExpressionHandler<'a, M, F>,
{
    fn from(expr: LowLevelILExpression<'a, M, F, bn::ValueExpr>) -> Self {
        use LowLevelILExpressionKind as Kind;
        match expr.kind() {
            Kind::Add(operation) => Expression::Add(Box::new(BinaryExpression::from(operation))),
            Kind::Sub(operation) => Expression::Sub(Box::new(BinaryExpression::from(operation))),
            Kind::And(operation) => Expression::And(Box::new(BinaryExpression::from(operation))),
            Kind::Xor(operation) => Expression::Xor(Box::new(BinaryExpression::from(operation))),
            Kind::Lsl(operation) => Expression::Lsl(Box::new(BinaryExpression::from(operation))),
            Kind::Lsr(operation) => Expression::Lsr(Box::new(BinaryExpression::from(operation))),
            Kind::CmpE(operation) => Expression::CmpE(Box::new(BinaryExpression(
                operation.left().into(),
                operation.right().into(),
            ))),
            Kind::Const(operation) => Expression::Const(operation.value()),
            Kind::ConstPtr(operation) => Expression::ConstPtr(operation.value()),
            Kind::Reg(operation) => Expression::Reg(operation.source_reg()),
            Kind::RegSsa(operation) => Expression::RegSsa(operation.source_reg()),
            _ => Expression::Unknown(expr),
        }
    }
}

impl<'a, M, F> From<bn::Operation<'a, M, F, bn::BinaryOp>> for BinaryExpression<'a, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'a, M, F, bn::ValueExpr>: bn::ExpressionHandler<'a, M, F>,
{
    fn from(operation: bn::Operation<'a, M, F, bn::BinaryOp>) -> Self {
        Self(operation.left().into(), operation.right().into())
    }
}
