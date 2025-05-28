use binaryninja::low_level_il::{
    expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
    function::{FunctionForm, FunctionMutability},
    instruction::{InstructionHandler, LowLevelILInstruction},
};
use bn_bdash_extras::llil::match_instr;

#[allow(dead_code, unused_variables)]
fn missing_wildcard_test<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    match_instr! {
        instr,
        Call(_) => Some("call"),
        SetRegSsa(_, _) => Some("set reg ssa")
        // Missing wildcard arm - should produce a clear error
    }
}

fn main() {} 
