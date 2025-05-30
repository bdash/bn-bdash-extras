use binaryninja::low_level_il::{
    expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
    function::{FunctionForm, FunctionMutability},
    instruction::{InstructionHandler, LowLevelILInstruction},
};
use bn_bdash_extras::llil::match_instr;

#[allow(dead_code)]
fn test_nested<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<String>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    match_instr! {
        instr,
        If(CmpE(And(Reg(reg), Const(mask)), Const(0)), true_target, false_target) => {
            Some(format!("if ({reg:?} & {mask:#x}) == 0, goto {true_target:?} else {false_target:?}"))
        },
        SetRegSsa(dest, Add(RegSsa(src), Const(const_))) if const_ % 2 == 0 => {
            Some(format!("{dest:?} = {src:?} + {const_}"))
        },
        _ => None,
    }
}

#[test]
fn test_match_instr() {
    // We're just testing that the macro compiles.
}
