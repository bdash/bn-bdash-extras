#![allow(dead_code, unused_variables)]

use binaryninja::{
    architecture::CoreRegister,
    low_level_il::{
        expression::{LowLevelILExpression, ValueExpr},
        function::{FunctionForm, FunctionMutability},
        instruction::LowLevelILInstruction,
        LowLevelILSSARegisterKind,
    },
};
use bn_bdash_extras::{
    llil::{
        BinaryExpression, Expression,
        ExpressionKind::{self, Const, RegSsa},
        Instruction,
        InstrMatch,
    },
};

#[derive(InstrMatch, Debug)]
#[pattern(instr @ SetRegSsa(dest, Lsr(RegSsa(source), Const(5))))]
struct SetToLsrBy5<'func, M, F>
where
    M: FunctionMutability,
    F: FunctionForm,
{
    instr: Instruction<'func, M, F>,
    dest: LowLevelILSSARegisterKind<CoreRegister>,
    source: LowLevelILSSARegisterKind<CoreRegister>,
}

#[derive(InstrMatch, Debug)]
#[pattern(SetRegSsa(_, Sub(Const(0x20), RegSsa(source))))]
struct SetToSubFrom32 {
    source: LowLevelILSSARegisterKind<CoreRegister>,
}

#[derive(InstrMatch, Debug)]
#[pattern(instr @ SetRegSsa(_, Const(0)))]
struct SetToConstZero<'func, M, F>
where
    M: FunctionMutability,
    F: FunctionForm,
{
    instr: LowLevelILInstruction<'func, M, F>,
}

#[derive(InstrMatch, Debug)]
#[pattern(instr @ SetRegSsa(_, Add(RegSsa(_), Const(1))))]
struct Increment<'func, M, F>
where
    M: FunctionMutability,
    F: FunctionForm,
{
    instr: LowLevelILInstruction<'func, M, F>,
}

#[derive(InstrMatch, Debug)]
#[pattern(instr @ SetRegSsa(_, lsr @ Lsr(reg_ssa @ RegSsa(_), const_ @ Const(1))))]
struct SetToLsrBy1<'func, M, F>
where
    M: FunctionMutability,
    F: FunctionForm,
{
    instr: LowLevelILInstruction<'func, M, F>,
    // Validate that expressions can be bound at each position in the expression, and are converted to the correct type.
    lsr: Expression<'func, M, F>,
    reg_ssa: ExpressionKind<'func, M, F>,
    const_: LowLevelILExpression<'func, M, F, ValueExpr>,
}

#[derive(InstrMatch, Debug)]
#[pattern(instr @ SetRegSsa(_, sub @ Sub(op)))]
struct SetToSub<'func, M, F>
where
    M: FunctionMutability,
    F: FunctionForm,
{
    instr: Instruction<'func, M, F>,
    sub: Expression<'func, M, F>,
    op: Box<BinaryExpression<'func, M, F>>,
}

#[test]
fn test_instr_match() {
    // We're just testing that the macro compiles.
}
