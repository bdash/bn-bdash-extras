use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, Pat, Result};

use super::analyze::{
    ArgumentPattern, ExpressionPattern, InstructionPattern, MatchArm, PatternKind,
    collect_pattern_bindings,
};

#[derive(Debug, Clone)]
pub struct ExpressionVar {
    pub sub_expr_var: Ident,
    pub expr_var: Ident,
    pub pattern: ExpressionPattern,
    pub nested_vars: Vec<ExpressionVar>,
    pub is_top_level: bool,
}

impl ExpressionVar {
    pub fn new(pattern: &ExpressionPattern, counter: &mut usize, is_top_level: bool) -> Self {
        let index = *counter;
        let mut nested_vars = Vec::new();
        for arg in &pattern.args {
            if let ArgumentPattern::Expression(nested_pattern) = arg {
                *counter += 1;
                nested_vars.push(ExpressionVar::new(nested_pattern, counter, false));
            }
        }

        ExpressionVar {
            sub_expr_var: format_ident!("__sub_expr_{}", index),
            expr_var: format_ident!("__expr_{}", index),
            pattern: pattern.clone(),
            nested_vars,
            is_top_level,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub arm_index: usize,
    pub bound_vars: Vec<BoundVariable>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct BoundVariable {
    pub ident: Ident,
    pub value_expr: TokenStream,
}

pub fn generate_match_code(expr: Expr, analyzed_arms: Vec<MatchArm>) -> Result<TokenStream> {
    let match_results = analyze_match_results(&analyzed_arms)?;
    let result_enum = generate_match_result_enum(&match_results)?;
    let state_machine_cases = generate_state_machine_cases(&match_results, &analyzed_arms)?;
    let final_match_cases = generate_final_match_cases(&match_results, &analyzed_arms);

    Ok(quote! {
        {
            #result_enum

            let __instruction = ::bn_bdash_extras::llil::InstructionKind::from(#expr);
            let __original = #expr;
            let mut __match_state = 0u64;

            let __match_result = loop {
                match __match_state {
                    #(#state_machine_cases),*
                }

                // Advance to next state if no pattern matched
                __match_state += 1;
            };

            match __match_result {
                #(#final_match_cases)*
                _ => unreachable!("Guards have already been checked"),
            }
        }
    })
}

/// Analyzes the match arms to collect information about the variables bound in each arm.
fn analyze_match_results(arms: &[MatchArm]) -> Result<Vec<MatchResult>> {
    let mut results = Vec::new();

    for (arm_index, arm) in arms.iter().enumerate() {
        match &arm.pattern_kind {
            PatternKind::Instruction(pattern) => {
                results.push(MatchResult {
                    arm_index,
                    bound_vars: collect_all_bound_variables(pattern),
                    body: arm.body.clone(),
                });
            }
            // Wildcard patterns have no bound variables
            PatternKind::Wildcard => {
                results.push(MatchResult {
                    arm_index,
                    bound_vars: vec![],
                    body: arm.body.clone(),
                });
            }
        }
    }

    Ok(results)
}

fn collect_all_bound_variables(pattern: &InstructionPattern) -> Vec<BoundVariable> {
    let mut bound_vars = Vec::new();

    if let Some(binding) = &pattern.binding {
        bound_vars.push(BoundVariable {
            ident: binding.clone(),
            value_expr: quote! { __original },
        });
    }

    for arg in &pattern.args {
        collect_vars_from_arg(arg, &mut bound_vars);
    }

    bound_vars
}

fn collect_vars_from_arg(arg: &ArgumentPattern, bound_vars: &mut Vec<BoundVariable>) {
    match arg {
        ArgumentPattern::Simple(pat) => {
            for binding in collect_pattern_bindings(pat) {
                bound_vars.push(BoundVariable {
                    ident: binding.clone(),
                    value_expr: quote! { #binding },
                });
            }
        }
        ArgumentPattern::Expression(expr_pattern) => {
            if let Some(binding) = &expr_pattern.binding {
                bound_vars.push(BoundVariable {
                    ident: binding.clone(),
                    value_expr: quote! { #binding },
                });
            }

            for sub_arg in &expr_pattern.args {
                collect_vars_from_arg(sub_arg, bound_vars);
            }
        }
    }
}

fn generate_match_result_enum(results: &[MatchResult]) -> Result<TokenStream> {
    let mut variants = Vec::new();
    let mut generic_params = Vec::new();
    let mut generic_counter = 0usize;

    for result in results {
        let variant_name = format_ident!("Arm{}", result.arm_index);

        let mut fields = Vec::new();
        for BoundVariable { ident, .. } in &result.bound_vars {
            let generic_name = format_ident!("__T{}", generic_counter);
            generic_counter += 1;
            generic_params.push(generic_name.clone());
            fields.push(quote! { #ident: #generic_name });
        }
        variants.push(quote! { #variant_name { #(#fields),* } });
    }

    let generics = if generic_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#generic_params),*> }
    };

    Ok(quote! {
        enum __MatchResult #generics {
            #(#variants),*
        }
    })
}

fn generate_state_machine_cases(
    results: &[MatchResult],
    arms: &[MatchArm],
) -> Result<Vec<TokenStream>> {
    arms.iter()
        .zip(results.iter())
        .enumerate()
        .map(|(arm_index, (arm, result))| {
            match &arm.pattern_kind {
                PatternKind::Instruction(pattern) => {
                    let (instruction_pattern, all_expr_vars) =
                        build_instruction_pattern(pattern, arm.guard.is_some())?;

                    generate_instruction_state_case(
                        arm_index,
                        instruction_pattern,
                        all_expr_vars,
                        arm.guard.clone(),
                        format_ident!("Arm{}", arm_index),
                        &result.bound_vars,
                    )
                }
                PatternKind::Wildcard => {
                    let variant_name = format_ident!("Arm{}", arm_index);
                    let success = quote! {
                        break __MatchResult::#variant_name {};
                    };

                    Ok(if let Some(g) = &arm.guard {
                        let state_value = arm_index as u64;
                        quote! { #state_value => if #g { #success } }
                    } else {
                        // For wildcard without guard (the final exhaustive case), use _
                        quote! { _ => { #success } }
                    })
                }
            }
        })
        .collect()
}

fn generate_instruction_state_case(
    state: usize,
    instruction_pattern: TokenStream,
    all_expr_vars: Vec<ExpressionVar>,
    guard: Option<Expr>,
    variant_name: Ident,
    bound_vars: &[BoundVariable],
) -> Result<TokenStream> {
    let fields = bound_vars
        .iter()
        .map(|BoundVariable { ident, value_expr }| quote! { #ident: #value_expr });
    let success = quote! { break __MatchResult::#variant_name { #(#fields),* }; };

    let innermost_body = if let Some(g) = guard {
        quote! { if #g { #success } }
    } else {
        quote! { { #success } }
    };

    // Bind `let` bindings for any top-level expression variables that need it.
    let top_level_bindings = all_expr_vars.iter().filter(|v| v.is_top_level).filter_map(
        |ExpressionVar { pattern, expr_var, .. }| {
            pattern
                .binding
                .as_ref()
                .map(|binding| quote! { let #binding = #expr_var.inner; })
        },
    );

    let body = wrap_with_expression_matches(
        quote! {
            #(#top_level_bindings)*
            #innermost_body
        },
        &all_expr_vars,
    );

    let state = state as u64;
    Ok(quote! {
        #state => {
            match __instruction {
                #instruction_pattern => { #body }
                _ => {}
            }
        }
    })
}

fn wrap_with_expression_matches(
    success_body: TokenStream,
    expr_vars: &[ExpressionVar],
) -> TokenStream {
    // Iterate in reverse order because fold builds the nested match structure from inside-out.
    expr_vars.iter().rev().fold(success_body, |acc, expr_var| {
        generate_expression_match(expr_var, acc)
    })
}

fn generate_expression_match(expr_var: &ExpressionVar, success_body: TokenStream) -> TokenStream {
    let content_var = &expr_var.sub_expr_var;
    let args = &expr_var.pattern.args;

    match args.as_slice() {
        [lhs, rhs] => {
            let mut nested_var_iter = expr_var.nested_vars.iter();
            let (lhs_needs_expr, lhs_pattern, lhs_binding) =
                generate_expression_pattern_and_bindings(lhs, &mut nested_var_iter);
            let (rhs_needs_expr, rhs_pattern, rhs_binding) =
                generate_expression_pattern_and_bindings(rhs, &mut nested_var_iter);

            let success_body = wrap_with_expression_matches(
                quote! {
                    #lhs_binding
                    #rhs_binding
                    #success_body
                },
                &expr_var.nested_vars,
            );

            let patterns = if lhs_needs_expr || rhs_needs_expr {
                // At least one side has nested expressions, match on the full Expression objects
                quote! { (#content_var.0.clone(), #content_var.1.clone()) }
            } else {
                quote! { #content_var.kinds() }
            };

            quote! {
                match #patterns {
                    (#lhs_pattern, #rhs_pattern) => {
                        #success_body
                    }
                    _ => {}
                }
            }
        }
        [arg] => {
            let mut nested_var_iter = expr_var.nested_vars.iter();
            let (_, arg_pattern, let_binding) =
                generate_expression_pattern_and_bindings(arg, &mut nested_var_iter);

            let success_body = wrap_with_expression_matches(
                quote! {
                    #let_binding
                    #success_body
                },
                &expr_var.nested_vars,
            );

            quote! {
                match #content_var.clone() {
                    #arg_pattern => { #success_body }
                    _ => {}
                }
            }
        }
        _ => success_body,
    }
}

/// Generate a pattern for an argument with bindings
fn generate_expression_pattern_and_bindings<'a>(
    arg: &ArgumentPattern,
    nested_var_iter: &mut impl Iterator<Item = &'a ExpressionVar>,
) -> (bool, TokenStream, Option<TokenStream>) {
    match arg {
        // Simple patterns don't need `Expression` matching
        ArgumentPattern::Simple(pat) => (false, quote! { #pat }, None),
        ArgumentPattern::Expression(ExpressionPattern { binding, type_name, .. }) => {
            let ExpressionVar {
                expr_var, sub_expr_var, ..
            } = nested_var_iter.next().unwrap();
            let (bind_expression, let_binding) = match &binding {
                Some(binding) => {
                    // Bind the `Expression` to an intermediate name, then use `let` to introduce
                    // a binding in the match arm to the inner `LowLevelILExpression`.
                    (
                        Some(quote! { ref #expr_var @ }),
                        Some(quote! { let #binding = #expr_var.inner; }),
                    )
                }
                None => (None, None),
            };

            let pattern = quote! {
                    #bind_expression ::bn_bdash_extras::llil::Expression {
                        kind: ::bn_bdash_extras::llil::ExpressionKind::#type_name(ref #sub_expr_var),
                        ..
                    }
            };

            // Expression patterns require matching on `Expression`, not `ExpressionKind`
            (true, pattern, let_binding)
        }
    }
}

fn build_instruction_pattern(
    pattern: &InstructionPattern,
    has_guard: bool,
) -> Result<(TokenStream, Vec<ExpressionVar>)> {
    let mut counter = 0;

    let (expr_vars, pattern_elements): (Vec<_>, Vec<_>) = pattern.args.iter().map(|arg| {
        match arg {
            ArgumentPattern::Simple(pat) => {
                // Bind by reference if a guard is present to avoid move issues if multiple arms
                // have guards involving the same pattern
                if has_guard && needs_ref(pat) {
                    (None, quote! { ref #pat })
                } else {
                    (None, quote! { #pat })
                }
            }
            ArgumentPattern::Expression(expr_pattern) => {
                let expr_var = ExpressionVar::new(expr_pattern, &mut counter, true);
                counter += 1;

                // Bind the expression variable if there's an @ binding
                let expr_var_name = &expr_var.expr_var;
                let bind_expression = expr_pattern
                    .binding
                    .as_ref()
                    .map(|_| quote! { ref #expr_var_name @ });

                let type_name = expr_pattern.type_name.clone();
                let content_var = expr_var.sub_expr_var.clone();

                (Some(expr_var), quote! {
                    #bind_expression ::bn_bdash_extras::llil::Expression {
                        kind: ::bn_bdash_extras::llil::ExpressionKind::#type_name(ref #content_var),
                        ..
                    }
                })

            }
        }
    }).unzip();

    let instr_name = &pattern.name;
    let full_pattern = quote! {
        ::bn_bdash_extras::llil::InstructionKind::#instr_name(#(#pattern_elements),*)
    };

    Ok((full_pattern, expr_vars.into_iter().flatten().collect()))
}

/// Whether a pattern needs a ref binding to avoid move issues
fn needs_ref(pat: &Pat) -> bool {
    match pat {
        Pat::Ident(ident_pat) => {
            // Simple identifiers need ref when they don't already have it
            ident_pat.by_ref.is_none()
                && ident_pat.mutability.is_none()
                && ident_pat.subpat.is_none()
        }
        _ => false,
    }
}

fn generate_final_match_cases(results: &[MatchResult], arms: &[MatchArm]) -> Vec<TokenStream> {
    results
        .iter()
        .zip(arms.iter())
        .map(|(result, arm)| {
            let variant_name = format_ident!("Arm{}", result.arm_index);
            let body = &result.body;

            let bindings = result.bound_vars.iter().map(|var| &var.ident);

            // Add the guard condition to ensure variables used only in guards are considered "used"
            let guard = arm
                .guard
                .as_ref()
                .map(|g| quote! { if true || #g })
                .unwrap_or_default();

            quote! {
                __MatchResult::#variant_name { #(#bindings),* } #guard => { #body },
            }
        })
        .collect()
}
