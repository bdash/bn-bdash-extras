//! Pattern-matchable wrappers around Binary Ninja's `low_level_il` types
//!
//! The medium- and high-level IL representation has a lifted variant that serves this purpose
//! match against, but that does not yet exist for low-level IL.
//!
//! This currently only supports the operations I've had a need to match against. It will need
//! to be expanded as it is used for more things.

use std::convert::Into;

use binaryninja::{
    architecture::CoreRegister,
    low_level_il::{
        self,
        expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
        instruction::{InstructionHandler, LowLevelILInstruction},
    },
};

#[derive(Debug)]
pub enum Instruction<'a, M, F>
where
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm,
{
    If(
        Expression<'a, M, F>,
        LowLevelILInstruction<'a, M, F>,
        LowLevelILInstruction<'a, M, F>,
    ),
    SetReg(
        low_level_il::LowLevelILRegisterKind<CoreRegister>,
        Expression<'a, M, F>,
    ),
    SetRegSsa(
        low_level_il::LowLevelILSSARegisterKind<CoreRegister>,
        Expression<'a, M, F>,
    ),
    RegPhi(
        low_level_il::LowLevelILSSARegisterKind<CoreRegister>,
        Vec<low_level_il::LowLevelILSSARegisterKind<CoreRegister>>,
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
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm;

#[derive(Debug)]
pub enum Expression<'a, M, F>
where
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm,
{
    Add(Box<BinaryExpression<'a, M, F>>),
    Sub(Box<BinaryExpression<'a, M, F>>),
    And(Box<BinaryExpression<'a, M, F>>),
    Xor(Box<BinaryExpression<'a, M, F>>),
    Lsl(Box<BinaryExpression<'a, M, F>>),
    Lsr(Box<BinaryExpression<'a, M, F>>),
    CmpE(Box<BinaryExpression<'a, M, F>>),
    Reg(low_level_il::LowLevelILRegisterKind<CoreRegister>),
    RegSsa(low_level_il::LowLevelILSSARegisterKind<CoreRegister>),
    Const(u64),
    ConstPtr(u64),
    Unknown(LowLevelILExpression<'a, M, F, ValueExpr>),
}

impl<'a, 'b, M, F> From<&'b LowLevelILInstruction<'a, M, F>> for Instruction<'b, M, F>
where
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm,
    LowLevelILInstruction<'a, M, F>: low_level_il::instruction::InstructionHandler<'a, M, F>,
    LowLevelILExpression<'a, M, F, ValueExpr>:
        low_level_il::expression::ExpressionHandler<'a, M, F>,
{
    fn from(instr: &'b LowLevelILInstruction<'a, M, F>) -> Self {
        use low_level_il::expression::LowLevelILExpressionKind as ExpressionKind;
        use low_level_il::instruction::LowLevelILInstructionKind as Kind;
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

impl<'a, M, F> From<LowLevelILExpression<'a, M, F, ValueExpr>> for Expression<'a, M, F>
where
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm,
    LowLevelILExpression<'a, M, F, ValueExpr>:
        low_level_il::expression::ExpressionHandler<'a, M, F>,
{
    fn from(expr: LowLevelILExpression<'a, M, F, ValueExpr>) -> Self {
        use low_level_il::expression::LowLevelILExpressionKind as Kind;
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

impl<'a, M, F> From<low_level_il::operation::Operation<'a, M, F, low_level_il::operation::BinaryOp>>
    for BinaryExpression<'a, M, F>
where
    M: low_level_il::function::FunctionMutability,
    F: low_level_il::function::FunctionForm,
    LowLevelILExpression<'a, M, F, ValueExpr>: ExpressionHandler<'a, M, F>,
{
    fn from(
        operation: low_level_il::operation::Operation<'a, M, F, low_level_il::operation::BinaryOp>,
    ) -> Self {
        Self(operation.left().into(), operation.right().into())
    }
}
