use binaryninja::low_level_il::{
    expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
    function::{FunctionForm, FunctionMutability},
    instruction::{InstructionHandler, LowLevelILInstruction},
};
use bn_bdash_extras::llil::{try_let_instr, ExpressionKind::*};

#[allow(dead_code, unused_variables)]
fn compile_test<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<()>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    try_let_instr! {
        let RegPhi(_, sources) = instr else { return None }
    };
    try_let_instr! {
        let SetRegSsa(dest, Add(RegSsa(src), Const(1))) = instr else { return None }
    };
    Some(())
}

#[test]
fn test_try_let_instr() {
    // We're just testing that the macro compiles.
}
