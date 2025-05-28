use binaryninja::low_level_il::{
    expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
    function::{FunctionForm, FunctionMutability},
    instruction::{InstructionHandler, LowLevelILInstruction},
};
use bn_bdash_extras::llil::match_instr;

#[allow(dead_code, unused_variables)]
fn test_malformed_pattern<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    let result = match_instr! {
        instr,
        // This should generate an error because std::collections::HashMap::new is a complex path
        Call(std::collections::HashMap::new(address)) => Some("malformed"),
        _ => None,
    };
    result
}

fn main() {} 
