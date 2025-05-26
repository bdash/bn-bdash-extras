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
pub struct Instruction<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    pub kind: InstructionKind<'func, M, F>,
    pub inner: LowLevelILInstruction<'func, M, F>,
}

#[derive(Debug)]
pub enum InstructionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    If(
        Expression<'func, M, F>,
        LowLevelILInstruction<'func, M, F>,
        LowLevelILInstruction<'func, M, F>,
    ),
    SetReg(
        LowLevelILRegisterKind<bn::CoreRegister>,
        Expression<'func, M, F>,
    ),
    SetRegSsa(
        LowLevelILSSARegisterKind<bn::CoreRegister>,
        Expression<'func, M, F>,
    ),
    RegPhi(
        LowLevelILSSARegisterKind<bn::CoreRegister>,
        Vec<LowLevelILSSARegisterKind<bn::CoreRegister>>,
    ),
    Call(Expression<'func, M, F>),
    TailCall(Expression<'func, M, F>),
    CallSsa(Expression<'func, M, F>, Vec<Expression<'func, M, F>>),
    TailCallSsa(Expression<'func, M, F>, Vec<Expression<'func, M, F>>),
    Goto(LowLevelILInstruction<'func, M, F>),
    Jump(Expression<'func, M, F>),
    Unknown(LowLevelILInstruction<'func, M, F>),
}

#[derive(Debug)]
pub struct BinaryExpression<'func, M, F>(pub Expression<'func, M, F>, pub Expression<'func, M, F>)
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm;

impl<'func, M, F> BinaryExpression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    pub fn kinds(&self) -> (&ExpressionKind<'func, M, F>, &ExpressionKind<'func, M, F>) {
        (&self.0.kind, &self.1.kind)
    }

    pub fn inners(&self) -> (
        LowLevelILExpression<'func, M, F, bn::ValueExpr>,
        LowLevelILExpression<'func, M, F, bn::ValueExpr>,
    ) {
        (self.0.inner, self.1.inner)
    }
}

#[derive(Debug)]
pub struct Expression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    pub inner: LowLevelILExpression<'func, M, F, bn::ValueExpr>,
    pub kind: ExpressionKind<'func, M, F>,
}

#[derive(Debug)]
pub enum ExpressionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
{
    Add(Box<BinaryExpression<'func, M, F>>),
    Sub(Box<BinaryExpression<'func, M, F>>),
    And(Box<BinaryExpression<'func, M, F>>),
    Xor(Box<BinaryExpression<'func, M, F>>),
    Lsl(Box<BinaryExpression<'func, M, F>>),
    Lsr(Box<BinaryExpression<'func, M, F>>),
    CmpE(Box<BinaryExpression<'func, M, F>>),
    Reg(LowLevelILRegisterKind<bn::CoreRegister>),
    RegSsa(LowLevelILSSARegisterKind<bn::CoreRegister>),
    Const(u64),
    ConstPtr(u64),
    Unknown(LowLevelILExpression<'func, M, F, bn::ValueExpr>),
}

impl<'func, M, F> From<LowLevelILInstruction<'func, M, F>> for Instruction<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(instr: LowLevelILInstruction<'func, M, F>) -> Self {
        Self {
            kind: InstructionKind::from(instr),
            inner: instr,
        }
    }
}

impl<'a, 'func, M, F> From<&'a LowLevelILInstruction<'func, M, F>> for Instruction<'func, M, F> 
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(instr: &'a LowLevelILInstruction<'func, M, F>) -> Self {
        instr.clone().into()
    }
}

impl<'func, M, F> From<LowLevelILInstruction<'func, M, F>>
    for InstructionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(instr: LowLevelILInstruction<'func, M, F>) -> Self {
        use LowLevelILExpressionKind as ExpressionKind;
        use LowLevelILInstructionKind as Kind;
        match instr.kind() {
            Kind::If(operation) => Self::If(
                operation.condition().into(),
                operation.true_target(),
                operation.false_target(),
            ),
            Kind::SetReg(operation) => {
                InstructionKind::SetReg(operation.dest_reg(), operation.source_expr().into())
            }
            Kind::SetRegSsa(operation) => {
                InstructionKind::SetRegSsa(operation.dest_reg(), operation.source_expr().into())
            }
            Kind::Call(operation) => InstructionKind::Call(operation.target().into()),
            Kind::TailCall(operation) => InstructionKind::TailCall(operation.target().into()),
            Kind::CallSsa(operation) => {
                let ExpressionKind::CallParamSsa(params) = operation.param_expr().kind() else {
                    panic!(
                        "Unexpected call parameter expression kind: {:?}",
                        operation.param_expr().kind()
                    );
                };
                InstructionKind::CallSsa(
                    operation.target().into(),
                    params.param_exprs().into_iter().map(|e| e.into()).collect(),
                )
            }
            Kind::TailCallSsa(operation) => {
                let ExpressionKind::CallParamSsa(params) = operation.param_expr().kind() else {
                    panic!(
                        "Unexpected call parameter expression kind: {:?}",
                        operation.param_expr().kind()
                    );
                };
                InstructionKind::TailCallSsa(
                    operation.target().into(),
                    params.param_exprs().into_iter().map(|e| e.into()).collect(),
                )
            }
            Kind::RegPhi(operation) => {
                let source_regs = operation.source_regs();
                InstructionKind::RegPhi(operation.dest_reg(), source_regs)
            }
            Kind::Goto(operation) => InstructionKind::Goto(operation.target()),
            Kind::Jump(operation) => InstructionKind::Jump(operation.target().into()),
            _ => InstructionKind::Unknown(instr),
        }
    }
}

impl<'a, 'func, M, F> From<&'a LowLevelILInstruction<'func, M, F>>
    for InstructionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(instr: &'a LowLevelILInstruction<'func, M, F>) -> Self {
        instr.clone().into()
    }
}

impl<'a, 'func, M, F> From<LowLevelILExpression<'func, M, F, bn::ValueExpr>>
    for Expression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(expr: LowLevelILExpression<'func, M, F, bn::ValueExpr>) -> Self {
        Self { inner: expr, kind: ExpressionKind::from(expr) }
    }
}

impl<'a, 'func, M, F> From<LowLevelILExpression<'func, M, F, bn::ValueExpr>>
    for ExpressionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(expr: LowLevelILExpression<'func, M, F, bn::ValueExpr>) -> Self {
        use LowLevelILExpressionKind as Kind;
        match expr.kind() {
            Kind::Add(operation) => ExpressionKind::Add(Box::new(operation.into())),
            Kind::Sub(operation) => ExpressionKind::Sub(Box::new(operation.into())),
            Kind::And(operation) => ExpressionKind::And(Box::new(operation.into())),
            Kind::Xor(operation) => ExpressionKind::Xor(Box::new(operation.into())),
            Kind::Lsl(operation) => ExpressionKind::Lsl(Box::new(operation.into())),
            Kind::Lsr(operation) => ExpressionKind::Lsr(Box::new(operation.into())),
            Kind::CmpE(operation) => ExpressionKind::CmpE(Box::new(BinaryExpression(
                operation.left().into(),
                operation.right().into(),
            ))),
            Kind::Const(operation) => ExpressionKind::Const(operation.value()),
            Kind::ConstPtr(operation) => ExpressionKind::ConstPtr(operation.value()),
            Kind::Reg(operation) => ExpressionKind::Reg(operation.source_reg()),
            Kind::RegSsa(operation) => ExpressionKind::RegSsa(operation.source_reg()),
            _ => ExpressionKind::Unknown(expr),
        }
    }
}

impl<'func, M, F> From<bn::Operation<'func, M, F, bn::BinaryOp>> for BinaryExpression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(operation: bn::Operation<'func, M, F, bn::BinaryOp>) -> Self {
        Self(operation.left().into(), operation.right().into())
    }
}
