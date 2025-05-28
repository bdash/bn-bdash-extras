//! Pattern-matchable wrappers around Binary Ninja's `low_level_il` types
//!
//! The medium- and high-level IL representation has a lifted variant that serves this purpose,
//! but that does not yet exist for low-level IL.
//!
//! This currently only supports the operations I've had a need to match against. It will need
//! to be expanded as it is used for more things.

use std::convert::Into;

use binaryninja::low_level_il::{
    expression::{ExpressionHandler as _, LowLevelILExpression, LowLevelILExpressionKind},
    instruction::{InstructionHandler as _, LowLevelILInstruction, LowLevelILInstructionKind},
    LowLevelILRegisterKind, LowLevelILSSARegisterKind,
};

mod bn {
    pub use binaryninja::{
        architecture::CoreRegister,
        architecture::Register,
        low_level_il::{
            expression::{ExpressionHandler, ValueExpr},
            function::{FunctionForm, FunctionMutability},
            instruction::{InstructionHandler, LowLevelILInstructionKind},
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
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    #[must_use]
    pub fn kinds(&self) -> (ExpressionKind<'func, M, F>, ExpressionKind<'func, M, F>) {
        (self.0.kind.clone(), self.1.kind.clone())
    }

    #[must_use]
    pub fn inners(
        &self,
    ) -> (
        LowLevelILExpression<'func, M, F, bn::ValueExpr>,
        LowLevelILExpression<'func, M, F, bn::ValueExpr>,
    ) {
        (self.0.inner, self.1.inner)
    }
}

impl<'func, M, F> Clone for BinaryExpression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
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

impl<'func, M, F> Clone for Expression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            kind: self.kind.clone(),
        }
    }
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

impl<'func, M, F> Clone for ExpressionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn clone(&self) -> Self {
        match self {
            ExpressionKind::Add(inner) => ExpressionKind::Add(inner.clone()),
            ExpressionKind::Sub(inner) => ExpressionKind::Sub(inner.clone()),
            ExpressionKind::And(inner) => ExpressionKind::And(inner.clone()),
            ExpressionKind::Xor(inner) => ExpressionKind::Xor(inner.clone()),
            ExpressionKind::Lsl(inner) => ExpressionKind::Lsl(inner.clone()),
            ExpressionKind::Lsr(inner) => ExpressionKind::Lsr(inner.clone()),
            ExpressionKind::CmpE(inner) => ExpressionKind::CmpE(inner.clone()),
            ExpressionKind::Reg(reg) => ExpressionKind::Reg(*reg),
            ExpressionKind::RegSsa(reg) => ExpressionKind::RegSsa(*reg),
            ExpressionKind::Const(value) => ExpressionKind::Const(*value),
            ExpressionKind::ConstPtr(value) => ExpressionKind::ConstPtr(*value),
            ExpressionKind::Unknown(expr) => ExpressionKind::Unknown(*expr),
        }
    }
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
        (*instr).into()
    }
}

impl<'func, M, F> From<LowLevelILInstruction<'func, M, F>> for InstructionKind<'func, M, F>
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
                InstructionKind::TailCallSsa(
                    operation.target().into(),
                    params.param_exprs().into_iter().map(Into::into).collect(),
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

impl<'a, 'func, M, F> From<&'a LowLevelILInstruction<'func, M, F>> for InstructionKind<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(instr: &'a LowLevelILInstruction<'func, M, F>) -> Self {
        (*instr).into()
    }
}

impl<'func, M, F> From<LowLevelILExpression<'func, M, F, bn::ValueExpr>> for Expression<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    fn from(expr: LowLevelILExpression<'func, M, F, bn::ValueExpr>) -> Self {
        Self {
            inner: expr,
            kind: ExpressionKind::from(expr),
        }
    }
}

impl<'func, M, F> From<LowLevelILExpression<'func, M, F, bn::ValueExpr>>
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

impl<'func, M, F> Instruction<'func, M, F>
where
    M: bn::FunctionMutability,
    F: bn::FunctionForm,
    LowLevelILInstruction<'func, M, F>: bn::InstructionHandler<'func, M, F>,
{
    #[must_use]
    pub fn size(&self) -> Option<usize> {
        use bn::LowLevelILInstructionKind as Kind;
        match self.inner.kind() {
            Kind::SetReg(ref op) => Some(op.size()),
            Kind::SetRegSsa(ref op) => Some(op.size()),
            Kind::Store(ref op) => Some(op.size()),
            Kind::StoreSsa(ref op) => Some(op.size()),
            Kind::Push(ref op) => Some(op.size()),
            _ => None,
        }
    }
}

/// Checks if two `LowLevelILSSARegisterKind` instances represent the same register, irrespective of their version.
pub fn is_same_register<R: bn::Register>(
    this: &LowLevelILSSARegisterKind<R>,
    other: &LowLevelILSSARegisterKind<R>,
) -> bool {
    match (this, other) {
        (
            LowLevelILSSARegisterKind::Full { kind: k1, .. },
            LowLevelILSSARegisterKind::Full { kind: k2, .. },
        ) => k1 == k2,
        (
            LowLevelILSSARegisterKind::Partial {
                full_reg: fr1,
                partial_reg: pr1,
                ..
            },
            LowLevelILSSARegisterKind::Partial {
                full_reg: fr2,
                partial_reg: pr2,
                ..
            },
        ) => fr1 == fr2 && pr1 == pr2,
        _ => false,
    }
}

/// Checks whether a given `LowLevelILSSARegisterKind` is a full register.
pub fn is_full_register<R: bn::Register>(reg: &LowLevelILSSARegisterKind<R>) -> bool {
    matches!(reg, LowLevelILSSARegisterKind::Full { .. })
}

/// Extract the underlying `LowLevelILRegisterKind` from a `LowLevelILSSARegisterKind` if it is a full register.
/// If not, logs a warning and returns `None`.
pub fn require_full_register<R: bn::Register, T: std::fmt::Debug>(
    reg: LowLevelILSSARegisterKind<R>,
    ctxt: &T,
) -> Option<LowLevelILRegisterKind<R>> {
    if let LowLevelILSSARegisterKind::Full { kind, .. } = reg {
        Some(kind)
    } else {
        log::warn!("Expected register used in {ctxt:?} to be a full SSA register, got {reg:?}");
        None
    }
}

/// A procedural macro for matching and destructuring of instructions and their expressions.
///
/// Provides pattern matching with support for nested expressions, variable bindings with `@`,
/// OR patterns, and guard conditions.
///
/// # Examples
/// ```no_run
/// # use bn_bdash_extras::llil::match_instr;
/// # use bn_bdash_extras::llil::{Instruction, ExpressionKind::*};
/// # let instr: binaryninja::low_level_il::instruction::LowLevelILInstruction<
/// #     binaryninja::low_level_il::function::Mutable,
/// #     binaryninja::low_level_il::function::SSA> = todo!();
/// match_instr!{
///     instr,
///     // Basic patterns
///     CallSsa(ConstPtr(address), _) => println!("Direct call to {:#x}", address),
///     
///     // Variable bindings and guards
///     instr @ SetRegSsa(dest, add @ Add(RegSsa(src), Const(value))) if value > 10 => {
///         println!(
///             "Increment of {src:?} by {value} > 10 at {:#x} (dest={dest:?}, add={add:?})",
///             instr.address(),
///         );
///     },
///     
///     // OR patterns
///     CallSsa(_, _) | TailCallSsa(_, _) => println!("Function call"),
///     
///     _ => {}
/// };
/// ```
#[doc(inline)]
pub use bn_bdash_extras_macros::match_instr;

/// Macro for pattern matching and destructuring of instructions and their expressions.
///
/// `try_let_instr!` allows you to match a [`LowLevelILInstruction`][binaryninja::low_level_il::instruction::LowLevelILInstruction]
/// against a single pattern, binding variables if the match succeeds and executing the `else` branch if the match fails.
/// It's analogous to `let-else` syntax in Rust, but allows matching through expressions in a single step.
/// This is useful for writing concise and readable instruction-matching code.
///
/// # Example
/// ```no_run
/// # use bn_bdash_extras::llil::try_let_instr;
/// # use bn_bdash_extras::llil::{Instruction, ExpressionKind::*};
/// # fn example<'func, M, F>(
/// #     instr: binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>,
/// # ) -> Option<()> where
/// #     M: binaryninja::low_level_il::function::FunctionMutability,
/// #     F: binaryninja::low_level_il::function::FunctionForm,
/// #     binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>:
/// #         binaryninja::low_level_il::instruction::InstructionHandler<'func, M, F>,
/// #     binaryninja::low_level_il::expression::LowLevelILExpression<'func, M, F, binaryninja::low_level_il::expression::ValueExpr>:
/// #         binaryninja::low_level_il::expression::ExpressionHandler<'func, M, F> {
/// try_let_instr!{
///     let SetRegSsa(dest, Lsr(RegSsa(source), Const(5))) = instr else { return None }
/// }
/// # Some(())
/// # }
/// ```
///
/// This expands to a match on `instr`, binding `dest` and `source` if the pattern matches,
/// or returning `None` if it does not.
#[doc(inline)]
pub use bn_bdash_extras_macros::try_let_instr;

/// Derive macro for implementing instruction pattern matching on structs.
///
/// This macro allows you to annotate a struct with a `#[pattern(...)]` attribute,
/// specifying a Rust pattern that matches a [`LowLevelILInstruction`].
///
/// An implementation of the [`TryFrom`] trait will be generated to support converting
/// from [`LowLevelILInstruction`]. This enables ergonomic and type-safe matching and
/// extraction of instructions and their subexpressions.
///
/// # Details
/// Bindings in the pattern correspond to the struct's fields. Each binding must have a matching field
/// of the appropriate type.
/// 
/// Fields are initialized using [`Into`] conversions from the matched instruction and expressions.
/// This allows you to bind an expression to a field of type [`Expression`], [`ExpressionKind`], or
/// [`LowLevelILExpression`] depending on your needs, and similarly for the instruction itself.
///
/// If the struct binds to an instruction or expression type that requires generic parameters,
/// struct must be defined with the necessary generic parameters for the object it captures.
/// For instructions and expressions this will often be the function mutability and form traits.
/// These MUST use the names `'func`, `M`, and `F` to avoid conflicting with the generated implementations.
///
/// # Example
/// ```no_run
/// # use binaryninja::{
/// #     architecture::CoreRegister,
/// #     low_level_il::{
/// #         function::{FunctionForm, FunctionMutability},
/// #         instruction::LowLevelILInstruction,
/// #         LowLevelILSSARegisterKind,
/// #     },
/// # };
/// # use bn_bdash_extras::{
/// #     llil::{
/// #         BinaryExpression, Expression,
/// #         ExpressionKind::{self, Const, RegSsa},
/// #         Instruction,
/// #         InstrMatch,
/// #     },
/// # };
/// #
/// #[derive(InstrMatch)]
/// #[pattern(instr @ SetRegSsa(dest, Lsr(RegSsa(source), Const(5))))]
/// struct SetToLsrBy5<'func, M, F>
///  where
///      M: FunctionMutability,
///      F: FunctionForm,
/// {
///     instr: LowLevelILInstruction<'func, M, F>,
///     dest: LowLevelILSSARegisterKind<CoreRegister>,
///     source: LowLevelILSSARegisterKind<CoreRegister>,
/// }
/// 
/// # fn example<'func, M, F>(
/// #     instr: LowLevelILInstruction<'func, M, F>,
/// # ) where
/// #     M: binaryninja::low_level_il::function::FunctionMutability,
/// #     F: binaryninja::low_level_il::function::FunctionForm,
/// #     LowLevelILInstruction<'func, M, F>: binaryninja::low_level_il::instruction::InstructionHandler<'func, M, F>,
/// #     binaryninja::low_level_il::expression::LowLevelILExpression<'func, M, F, binaryninja::low_level_il::expression::ValueExpr>:
/// #         binaryninja::low_level_il::expression::ExpressionHandler<'func, M, F> {
/// // Usage:
/// let match_result = SetToLsrBy5::try_from(instr);
/// if let Ok(matched) = match_result {
///     println!(
///         "Matched SetRegSsa({:?}, Lsr(RegSsa({:?}), Const(5))) at address: {:#x}",
///         matched.dest, matched.source, matched.instr.address(),
///     );
///     // You can now use `matched.instr` and other fields as needed.
/// } else {
///     println!("Instruction did not match");
/// }
/// # }
/// ```
///
#[doc(inline)]
pub use bn_bdash_extras_macros::InstrMatch;
