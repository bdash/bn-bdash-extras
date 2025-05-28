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
    pub args: Vec<Pat>,
}

impl ExpressionVar {
    pub fn new(index: usize, pattern: &ExpressionPattern) -> Self {
        ExpressionVar {
            sub_expr_var: format_ident!("__sub_expr_{}", index),
            expr_var: format_ident!("__expr_{}", index),
            args: pattern.args.clone(),
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
    let state_enum = generate_match_state_enum(&match_results)?;
    let result_enum = generate_match_result_enum(&match_results)?;
    let state_machine_cases = generate_state_machine_cases(&match_results, &analyzed_arms)?;
    let final_match_cases = generate_final_match_cases(&match_results)?;

    Ok(quote! {
        {
            #state_enum
            #result_enum

            let __instruction = ::bn_bdash_extras::llil::InstructionKind::from(#expr);
            let __original = #expr;
            let mut __match_state = __MatchState::MatchState0;

            let __match_result = loop {
                match __match_state {
                    #(#state_machine_cases)*
                }
            };

            match __match_result {
                #(#final_match_cases)*
            }
        }
    })
}

/// Analyzes the match arms to collect information about the variables bound in each arm.
fn analyze_match_results(arms: &[MatchArm]) -> Result<Vec<MatchResult>> {
    let mut results = Vec::new();

    for (arm_index, arm) in arms.iter().enumerate() {
        let mut bound_vars = Vec::new();

        match &arm.pattern_kind {
            PatternKind::Instruction(pattern) => {
                // Add a binding for the instruction itself, if present
                if let Some(binding) = &pattern.binding {
                    bound_vars.push(BoundVariable {
                        ident: binding.clone(),
                        value_expr: quote! { __original },
                    });
                }

                // Add any bindings for the expressions within the instruction
                for (i, arg) in pattern.args.iter().enumerate() {
                    match arg {
                        ArgumentPattern::Expression(expr_pattern) => {
                            let expr_var = format_ident!("__expr_{}", i);
                            if let Some(binding) = &expr_pattern.binding {
                                bound_vars.push(BoundVariable {
                                    ident: binding.clone(),
                                    value_expr: quote! { #expr_var.inner },
                                });
                            }
                            // Add any bindings for the subexpressions within the expression
                            for arg_pat in &expr_pattern.args {
                                add_bound_variables(arg_pat, &mut bound_vars);
                            }
                        }
                        ArgumentPattern::Simple(pat) => {
                            add_bound_variables(pat, &mut bound_vars);
                        }
                    }
                }
            }
            // Wildcard patterns have no bound variables
            PatternKind::Wildcard => {}
        }

        results.push(MatchResult {
            arm_index,
            bound_vars,
            body: arm.body.clone(),
        });
    }

    Ok(results)
}

fn add_bound_variables(pat: &Pat, bound_vars: &mut Vec<BoundVariable>) {
    for binding_ident in collect_pattern_bindings(pat) {
        bound_vars.push(BoundVariable {
            ident: binding_ident.clone(),
            value_expr: quote! { #binding_ident },
        });
    }
}

fn generate_match_result_enum(results: &[MatchResult]) -> Result<TokenStream> {
    let mut variants = Vec::new();
    let mut generic_params = Vec::new();
    let mut generic_counter = 0usize;

    for result in results {
        let variant_name = format_ident!("Arm{}", result.arm_index);

        let mut fields = Vec::new();
        for bound_var in &result.bound_vars {
            let field_name = &bound_var.ident;
            let generic_name = format_ident!("__T{}", generic_counter);
            generic_params.push(generic_name.clone());
            fields.push(quote! { #field_name: #generic_name });
            generic_counter += 1;
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

fn generate_match_state_enum(results: &[MatchResult]) -> Result<TokenStream> {
    let variants = results
        .iter()
        .map(|result| format_ident!("MatchState{}", result.arm_index));

    Ok(quote! {
        enum __MatchState {
            #(#variants),*
        }
    })
}

fn generate_state_machine_cases(
    results: &[MatchResult],
    arms: &[MatchArm],
) -> Result<Vec<TokenStream>> {
    let mut states = Vec::new();

    for (result, arm) in results.iter().zip(arms.iter()) {
        let variant_name = format_ident!("Arm{}", result.arm_index);

        match &arm.pattern_kind {
            PatternKind::Instruction(pattern) => {
                let (instruction_pattern, expr_vars) =
                    build_instruction_pattern(pattern, arm.guard.is_some())?;
                let guard = arm
                    .guard
                    .as_ref()
                    .map(|g| quote! { if #g })
                    .unwrap_or_default();

                let fields = result.bound_vars.iter().map(|var| {
                    let field_name = &var.ident;
                    let value_expr = &var.value_expr;
                    quote! { #field_name: #value_expr }
                });
                let match_result = quote! { __MatchResult::#variant_name { #(#fields),* } };

                let state = generate_instruction_state_case(
                    result.arm_index,
                    instruction_pattern,
                    expr_vars,
                    guard,
                    match_result,
                )?;
                states.push(state);
            }
            PatternKind::Wildcard => {
                let guard = arm
                    .guard
                    .as_ref()
                    .map(|g| quote! { if #g })
                    .unwrap_or_default();

                let current_state_variant = format_ident!("MatchState{}", result.arm_index);
                let state = quote! {
                    __MatchState::#current_state_variant => #guard {
                        break __MatchResult::#variant_name { };
                    }
                };
                states.push(state);
            }
        }
    }
    Ok(states)
}

fn generate_instruction_state_case(
    state: usize,
    instruction_pattern: TokenStream,
    expr_vars: Vec<ExpressionVar>,
    guard: TokenStream,
    result: TokenStream,
) -> Result<TokenStream> {
    let current_state_variant = format_ident!("MatchState{}", state);
    let next_state_variant = format_ident!("MatchState{}", state + 1);
    let fallback = quote! { __match_state = __MatchState::#next_state_variant; continue; };

    // Check guard after expression matching, when all bound variables are in scope
    let result_with_guard = if guard.is_empty() {
        result
    } else {
        quote! {
            #guard {
                #result
            } else {
                #fallback
            }
        }
    };

    let result_if_expressions_match =
        wrap_with_expression_matches(result_with_guard, &expr_vars, fallback.clone());

    Ok(quote! {
        __MatchState::#current_state_variant => {
            match __instruction {
                #instruction_pattern => break #result_if_expressions_match,
                _ => { #fallback }
            }
        }
    })
}

fn wrap_with_expression_matches(
    success_body: TokenStream,
    expr_vars: &[ExpressionVar],
    fallback: TokenStream,
) -> TokenStream {
    // Iterate in reverse order because fold builds the nested match structure from inside-out.
    expr_vars.iter().rev().fold(success_body, |acc, expr_var| {
        generate_expression_match(expr_var, acc, fallback.clone())
    })
}

fn generate_expression_match(
    expr_var: &ExpressionVar,
    success_body: TokenStream,
    fallback: TokenStream,
) -> TokenStream {
    let content_var = &expr_var.sub_expr_var;

    match expr_var.args.as_slice() {
        [lhs, rhs] => {
            let lhs_subexpr_bind =
                generate_subexpr_bind_if_needed(lhs, quote! { #content_var.inners().0 }, "lhs");
            let rhs_subexpr_bind =
                generate_subexpr_bind_if_needed(rhs, quote! { #content_var.inners().1 }, "rhs");

            quote! {
                match #content_var.kinds() {
                    (#lhs, #rhs) => {
                        #lhs_subexpr_bind
                        #rhs_subexpr_bind
                        #success_body
                    }
                    _ => { #fallback }
                }
            }
        }
        [arg] => {
            // For single argument expressions, the argument is not a subexpression
            quote! {
                match #content_var.clone() {
                    #arg => { #success_body }
                    _ => { #fallback }
                }
            }
        }
        _ => success_body,
    }
}

/// Generates setup code for subexpression bindings
fn generate_subexpr_bind_if_needed(
    pat: &Pat,
    expr_access: TokenStream,
    _position: &str,
) -> TokenStream {
    match pat {
        Pat::Ident(ident_pat) if ident_pat.subpat.is_some() => {
            let binding_ident = &ident_pat.ident;
            if let Some((_, subpat)) = &ident_pat.subpat {
                if let Pat::TupleStruct(_) = subpat.as_ref() {
                    // Create the variable used for binding the subexpression.
                    return quote! { let #binding_ident = #expr_access; };
                }
            }
        }
        _ => {}
    }
    quote! {}
}

fn build_instruction_pattern(
    pattern: &InstructionPattern,
    has_guard: bool,
) -> Result<(TokenStream, Vec<ExpressionVar>)> {
    let mut pattern_elements = Vec::new();
    let mut expr_vars = Vec::new();

    for (i, arg) in pattern.args.iter().enumerate() {
        match arg {
            ArgumentPattern::Simple(pat) => {
                // Use ref bind if guard is present to avoid move issues if multiple arms have guards involving the same pattern
                let pattern_to_use = if has_guard && needs_ref(pat) {
                    quote! { ref #pat }
                } else {
                    quote! { #pat }
                };
                pattern_elements.push(pattern_to_use);
            }
            ArgumentPattern::Expression(expr_pattern) => {
                let expr_var = ExpressionVar::new(i, expr_pattern);
                let type_name = &expr_pattern.type_name;
                let content_var = &expr_var.sub_expr_var;
                let expr_var_name = &expr_var.expr_var;

                pattern_elements.push(quote! {
                    ref #expr_var_name @ ::bn_bdash_extras::llil::Expression {
                        kind: ::bn_bdash_extras::llil::ExpressionKind::#type_name(ref #content_var),
                        ..
                    }
                });

                expr_vars.push(expr_var);
            }
        }
    }

    let instr_name = &pattern.name;
    let full_pattern = quote! {
        ::bn_bdash_extras::llil::InstructionKind::#instr_name(#(#pattern_elements),*)
    };

    Ok((full_pattern, expr_vars))
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

fn generate_final_match_cases(results: &[MatchResult]) -> Result<Vec<TokenStream>> {
    let mut cases = Vec::new();

    for result in results {
        let variant_name = format_ident!("Arm{}", result.arm_index);
        let body = &result.body;

        let bindings = result.bound_vars.iter().map(|var| &var.ident);
        cases.push(quote! {
            __MatchResult::#variant_name { #(#bindings),* } => { #body }
        });
    }

    Ok(cases)
}
