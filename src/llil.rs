//! Pattern-matchable wrappers around Binary Ninja's `low_level_il` types
//!
//! The medium- and high-level IL representation has a lifted variant that serves this purpose,
//! but that does not yet exist for low-level IL.
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
    LowLevelILExpression<'func, M, F, bn::ValueExpr>: bn::ExpressionHandler<'func, M, F>,
{
    pub fn kinds(&self) -> (ExpressionKind<'func, M, F>, ExpressionKind<'func, M, F>) {
        (self.0.kind.clone(), self.1.kind.clone())
    }

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
            inner: self.inner.clone(),
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
            ExpressionKind::Unknown(expr) => ExpressionKind::Unknown(expr.clone()),
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
        instr.clone().into()
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

impl<'a, 'func, M, F> From<&'a LowLevelILInstruction<'func, M, F>> for InstructionKind<'func, M, F>
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
        Self {
            inner: expr,
            kind: ExpressionKind::from(expr),
        }
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

#[cfg(feature = "macros")]
pub mod macros {
    #[doc(inline)]
    pub use crate::__match_instr as match_instr;

    /// Macro for pattern matching and destructuring of instructions and their expressions.
    ///
    /// `try_let_instr!` allows you to match a [`LowLevelILInstruction`][binaryninja::low_level_il::instruction::LowLevelILInstruction]
    /// against a single pattern, binding variables if the match succeeds and executing the `else` branch if the match fails.
    /// It's analogous to `let-else` syntax in Rust, but allows matching through expressions in a single step.
    /// This is useful for writing concise and readable instruction-matching code.
    ///
    /// # Example
    /// ```ignore
    /// try_let_instr!{
    ///     let SetRegSsa(dest, Lsr(RegSsa(source), Const(5))) = instr else { return None }
    /// }
    /// ```
    ///
    /// This expands to a match on `instr`, binding `dest` and `source` if the pattern matches,
    /// or returning `None` if it does not.
    pub use bn_bdash_extras_macros::try_let_instr;

    /// Derive macro for implementing instruction pattern matching on structs.
    ///
    /// This macro allows you to annotate a struct with a `#[pattern(...)]` attribute,
    /// specifying a Rust pattern that matches a [`LowLevelILInstruction`][binaryninja::low_level_il::instruction::LowLevelILInstruction].
    ///
    /// The macro generates an implementation of the [`TryFrom`] trait for the struct, converting from
    /// [`LowLevelILInstruction<'func, M, F>`][binaryninja::low_level_il::instruction::LowLevelILInstruction] enabling ergonomic and
    /// type-safe matching and extraction of instructions and their subexpressions.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(InstrMatch)]
    /// #[pattern(instr @ SetRegSsa(dest, Lsr(RegSsa(source), Const(5))))]
    /// struct SetToLsrBy5<'func, M, F>
    ///  where
    ///      M: binaryninja::low_level_il::function::FunctionMutability,
    ///      F: binaryninja::low_level_il::function::FunctionForm,
    /// {
    ///     instr: LowLevelILInstruction<'func, M, F>,
    ///     dest: LowLevelILSSARegisterKind<CoreRegister>,
    ///     source: LowLevelILSSARegisterKind<CoreRegister>,
    /// }
    /// ```
    ///
    /// The struct fields must correspond to the bindings in the pattern.
    ///
    /// If the struct binds to the instruction itself via `binding @`, the struct must be defined
    /// with the necessary generic parameters for the `LowLevelILInstruction` it captures.
    /// They MUST use the names `'func`, `M`, and `F` to avoid conflicting with the generated implementations.
    pub use bn_bdash_extras_macros::InstrMatch;

    /// Macro for ergonomic pattern matching on instructions and their subexpressions.
    ///
    /// `match_instr!` allows matching on the structure of a [`LowLevelILInstruction`][binaryninja::low_level_il::instruction::LowLevelILInstruction]
    /// using Rust patterns that correspond to the enum variants of [`InstructionKind`][super::InstructionKind] and
    /// [`ExpressionKind`][super::ExpressionKind].
    ///
    /// Unlike a normal Rust `match`, this macro allows matching an instruction and its subexpressions
    /// in a single pattern, rather than requiring a multi-statement destructuring process.
    /// This enables concise, readable, and expressive matching for complex instruction trees.
    ///
    /// # Example
    /// ```ignore
    /// match_instr! {
    ///     instr,
    ///     SetRegSsa(dest, Lsr(RegSsa(source), Const(5))) => {
    ///         // handle logical shift right by 5
    ///         ...
    ///     },
    ///     SetRegSsa(_, RegSsa(r)) => {
    ///         // handle special register
    ///         ...
    ///     },
    ///     SetRegSsa(dest, Lsr(RegSsa(source), Const(value))) if value > 8 => {
    ///        // handle large logical shift right
    ///       ...
    ///     },
    ///     Goto(target) => {
    ///         // handle goto
    ///         ...
    ///     },
    ///     _ => {
    ///         // fallback
    ///         ...
    ///     },
    /// }
    /// ```
    #[doc(hidden)]
    #[macro_export]
    macro_rules! __match_instr {
    // Entry point: evaluate the expression once, bind to __val, then dispatch to @internal
    ($Expr:expr, $($rest:tt)*) => {{
        let __val = $crate::llil::InstructionKind::from($Expr);
        let __orig = $Expr;
        $crate::__match_instr!(@internal __val, __orig, $($rest)*)
    }};

    // 1) Final wildcard arm
    (@internal $val:ident, $orig:ident, _ => $default:expr $(,)?) => {
        $default
    };

    // 2a) @ binding with Instr(instr, Expr(lhs, rhs)) if guard =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($i_pat:pat, $Expr:ident($lpat:pat, $rpat:pat)) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.kinds() {
                    ($lpat, $rpat) if $user_guard => {
                        let $bind = $orig;
                        $body
                    },
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 2b) @ binding with Instr(instr, Expr(lhs, rhs)) =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($i_pat:pat, $Expr:ident($lpat:pat, $rpat:pat)) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.kinds() {
                    ($lpat, $rpat) => {
                        let $bind = $orig;
                        $body
                    },
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 2c) @ binding with Instr(instr, Expr(expr)) if guard =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($i_pat:pat, $Expr:ident($opat:pat)) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.clone() {
                    $opat if $user_guard => {
                        let $bind = $orig;
                        $body
                    },
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 2d) @ binding with Instr(instr, Expr(expr)) =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($i_pat:pat, $Expr:ident($opat:pat)) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.clone() {
                    $opat => {
                        let $bind = $orig;
                        $body
                    },
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 2e) @ binding with Instr(instr) if guard =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($($ppats:pat),*) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($($ppats),*) if $user_guard => {
                let $bind = $orig;
                $body
            },
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 2f) @ binding with Instr(instr) =>
    (@internal $val:ident, $orig:ident,
        $bind:ident @ $Inst:ident($($ppats:pat),*) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($($ppats),*) => {
                let $bind = $orig;
                $body
            },
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3a) Instr(instr, Expr(lhs, rhs)) if guard =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($i_pat:pat, $Expr:ident($lpat:pat, $rpat:pat)) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.kinds() {
                    ($lpat, $rpat) if $user_guard => $body,
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3b) Instr(instr, Expr(lhs, rhs)) =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($i_pat:pat, $Expr:ident($lpat:pat, $rpat:pat)) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.kinds() {
                    ($lpat, $rpat) => $body,
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3c) Instr(instr, Expr(expr)) if guard =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($i_pat:pat, $Expr:ident($opat:pat)) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.clone() {
                    $opat if $user_guard => $body,
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3d) Instr(instr, Expr(expr)) =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($i_pat:pat, $Expr:ident($opat:pat)) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($i_pat, $crate::llil::Expression { kind: $crate::llil::ExpressionKind::$Expr(ref inner), .. }) => {
                match inner.clone() {
                    $opat => $body,
                    _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
                }
            }
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3e) Instr(instr) if guard =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($($ppats:pat),*) if $user_guard:expr => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($($ppats),*) if $user_guard => $body,
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

    // 3f) Instr(instr) =>
    (@internal $val:ident, $orig:ident,
        $Inst:ident($($ppats:pat),*) => $body:expr,
        $($rest:tt)*
    ) => {
        match $val {
            $crate::llil::InstructionKind::$Inst($($ppats),*) => $body,
            _ => $crate::__match_instr!(@internal $val, $orig, $($rest)*),
        }
    };

}
}
