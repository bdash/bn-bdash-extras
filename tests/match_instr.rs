use binaryninja::{
    architecture::CoreRegister,
    low_level_il::{
        expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
        function::{FunctionForm, FunctionMutability},
        instruction::{InstructionHandler, LowLevelILInstruction},
        LowLevelILSSARegisterKind,
    },
};
use bn_bdash_extras::llil::{match_instr, Expression, ExpressionKind::*, Instruction};

#[allow(dead_code, unused_variables)]
fn compile_test<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    let result = match_instr! {
        instr,
        RegPhi(_, sources) => {
            Some("reg_phi")
        },
        SetRegSsa(dest, Add(RegSsa(src), Const(1))) => {
            Some("set reg ssa")
        },
        SetRegSsa(dest, Sub(RegSsa(src), Const(c))) if c > 0xffff => {
            Some("set reg ssa sub (big)")
        },
        SetRegSsa(dest, Sub(RegSsa(src), Const(c))) if c > 0xff => {
            Some("set reg ssa sub (small)")
        },
        TailCall(target) => Some("tail call"),
        Call(ConstPtr(address)) => Some("call"),
        CallSsa(ConstPtr(address), _) => Some("call ssa"),
        If(Const(0), _, _) => Some("if"),
        TailCallSsa(target @ ConstPtr(address), _) => {
            let _ = target;
            Some("tail call ssa")
        },
        _ => return None,
    };
    result
}

#[allow(dead_code, unused_variables)]
fn match_within_loop<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    #[allow(clippy::never_loop)]
    let result = loop {
        let result = match_instr! {
            instr,
            RegPhi(_, sources) => break Some("reg_phi"),
            SetRegSsa(dest, Add(lhs @ RegSsa(src), Const(1))) => Some("set reg ssa"),
            instr @ Call(target) => continue,
            _ => break None,
        };
        return result;
    };
    result
}

#[allow(dead_code, unused_variables)]
fn extra_binding_test<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>)
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    match_instr! {
        instr,
        instr @ SetRegSsa(dest, add @ Add(reg_ssa @ RegSsa(src), const_ @ Const(1))) => {
            let _ : Instruction<'func, M, F> = instr.into();
            let _ : Expression<'func, M, F> = add.into();
            let _ : Expression<'func, M, F> = reg_ssa.into();
            let _ : Expression<'func, M, F> = const_.into();
        },
        instr @ SetRegSsa(dest, reg_ssa @ RegSsa(reg)) => {
            let _ : Instruction<'func, M, F> = instr.into();
            let _ : Expression<'func, M, F> = reg_ssa.into();
            let _ : LowLevelILSSARegisterKind<CoreRegister> = reg;
        },
        instr @ Call(target @ ConstPtr(address)) => {
            let _ : Instruction<'func, M, F> = instr.into();
            let _ : Expression<'func, M, F> = target.into();
            let _ : u64 = address;
        },
        _ => {}
    };
}

#[test]
fn test_match_instr() {
    // We're just testing that the macro compiles.
}
