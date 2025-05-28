use binaryninja::low_level_il::{
    expression::{ExpressionHandler, LowLevelILExpression, ValueExpr},
    function::{FunctionForm, FunctionMutability},
    instruction::{InstructionHandler, LowLevelILInstruction},
};

use bn_bdash_extras::llil::match_instr;

#[allow(dead_code, unused_variables)]
fn test_or_patterns<'func, M, F>(instr: LowLevelILInstruction<'func, M, F>) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    match_instr! {
        instr,
        // Test basic OR patterns
        Call(_) | TailCall(_) => Some("function call"),

        // Test OR patterns with bindings
        SetRegSsa(dest, _) | SetReg(dest, _) => {
            let _ = dest;
            Some("register assignment")
        },

        // Test OR patterns with @ bindings
        instr_ref @ CallSsa(_, _) | instr_ref @ TailCallSsa(_, _) => {
            let _ = instr_ref;
            Some("ssa call")
        },

        // Test OR patterns with complex expression matching
        SetRegSsa(_, RegSsa(_)) | SetReg(_, Reg(_)) => Some("register copy"),

        // Test OR patterns with wildcard
        If(_, _, _) | Goto(_) | Jump(_) => Some("control flow"),

        _ => None
    }
}

#[allow(dead_code, unused_variables)]
fn test_or_patterns_with_guards<'func, M, F>(
    instr: LowLevelILInstruction<'func, M, F>,
) -> Option<&'static str>
where
    M: FunctionMutability,
    F: FunctionForm,
    LowLevelILInstruction<'func, M, F>: InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, ValueExpr>: ExpressionHandler<'func, M, F>,
{
    match_instr! {
        instr,
        // Test OR patterns with guards
        Call(target) | TailCall(target) if true => {
            let _ = target;
            Some("guarded call")
        },

        Call(_) | TailCall(_) => Some("unguarded call"),

        _ => None
    }
}

#[test]
fn test_or_patterns_compile() {
    // We're just testing that the OR patterns compile successfully.
}
