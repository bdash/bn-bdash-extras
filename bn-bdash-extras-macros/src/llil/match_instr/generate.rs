use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, Pat};

use super::analyze::{
    ArgumentPattern, ExpressionPattern, InstructionPattern, MatchArm, PatternKind,
    collect_pattern_bindings,
};

#[derive(Debug, Clone)]
pub struct ExpressionVarDetails {
    pub expr_var: Ident,
    pub sub_expr_var: Ident,
    pub pattern: ExpressionPattern,
    pub nested_vars: Vec<PatternVar>,
}

pub struct IdSource(std::ops::RangeFrom<usize>);

impl IdSource {
    fn next(&mut self) -> usize {
        self.0.next().unwrap()
    }
}

impl ExpressionVarDetails {
    pub fn new(source_pattern: &ExpressionPattern, id_source: &mut IdSource) -> Self {
        let id = id_source.next();

        let nested_vars = source_pattern
            .args
            .iter()
            .filter(|arg| PatternVar::needs_var(arg))
            .map(|arg| PatternVar::new(arg, id_source))
            .collect();

        ExpressionVarDetails {
            expr_var: format_ident!("__expr_{id}"),
            sub_expr_var: format_ident!("__sub_expr_{id}"),
            pattern: source_pattern.clone(),
            nested_vars,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PatternVar {
    Simple { expr_var: Ident, pattern: Pat },
    Expression(ExpressionVarDetails),
}

impl PatternVar {
    pub fn new(pattern_arg: &ArgumentPattern, id_source: &mut IdSource) -> Self {
        match pattern_arg {
            ArgumentPattern::Simple(pat) => PatternVar::Simple {
                expr_var: format_ident!("__expr_{}", id_source.next()),
                pattern: pat.clone(),
            },
            ArgumentPattern::Expression(expr_data) => {
                PatternVar::Expression(ExpressionVarDetails::new(expr_data, id_source))
            }
        }
    }

    fn needs_var(arg: &ArgumentPattern) -> bool {
        match arg {
            ArgumentPattern::Simple(pat) => is_variable_binding(pat),
            ArgumentPattern::Expression(_) => true,
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

pub fn generate_match_code(expr: Expr, analyzed_arms: Vec<MatchArm>) -> TokenStream {
    let match_results = analyze_match_results(&analyzed_arms);
    let result_enum = generate_match_result_enum(&match_results);
    let state_machine_cases = generate_state_machine_cases(&match_results, &analyzed_arms);
    let final_match_cases = generate_final_match_cases(&match_results, &analyzed_arms);

    quote! {
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
    }
}

fn analyze_match_results(arms: &[MatchArm]) -> Vec<MatchResult> {
    arms.iter()
        .enumerate()
        .map(|(arm_index, arm)| match &arm.pattern_kind {
            PatternKind::Instruction(pattern) => MatchResult {
                arm_index,
                bound_vars: collect_all_bound_variables(pattern),
                body: arm.body.clone(),
            },
            PatternKind::Wildcard => MatchResult {
                arm_index,
                bound_vars: vec![],
                body: arm.body.clone(),
            },
        })
        .collect()
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

fn generate_match_result_enum(results: &[MatchResult]) -> TokenStream {
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

    quote! {
        enum __MatchResult #generics {
            #(#variants),*
        }
    }
}

fn generate_state_machine_cases(results: &[MatchResult], arms: &[MatchArm]) -> Vec<TokenStream> {
    arms.iter()
        .zip(results.iter())
        .enumerate()
        .map(|(arm_index, (arm, result))| {
            match &arm.pattern_kind {
                PatternKind::Instruction(pattern) => {
                    let (instruction_pattern, all_expr_vars) =
                        build_instruction_pattern(pattern, arm.guard.is_some());

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

                    if let Some(g) = &arm.guard {
                        let state_value = arm_index as u64;
                        quote! { #state_value => if #g { #success } }
                    } else {
                        // For wildcard without guard (the final exhaustive case), use _
                        quote! { _ => { #success } }
                    }
                }
            }
        })
        .collect()
}

fn generate_instruction_state_case(
    state: usize,
    instruction_pattern: TokenStream,
    all_expr_vars: Vec<PatternVar>,
    guard: Option<Expr>,
    variant_name: Ident,
    bound_vars: &[BoundVariable],
) -> TokenStream {
    let fields = bound_vars
        .iter()
        .map(|BoundVariable { ident, value_expr }| quote! { #ident: #value_expr });
    let success = quote! { break __MatchResult::#variant_name { #(#fields),* }; };

    let innermost_body = if let Some(g) = guard {
        quote! { if #g { #success } }
    } else {
        quote! { { #success } }
    };

    // Generate all let bindings at the innermost level
    let all_bindings = collect_all_let_bindings(&all_expr_vars);

    let body = wrap_with_expression_matches(
        quote! {
            #(#all_bindings)*
            #innermost_body
        },
        &all_expr_vars,
    );

    let state = state as u64;
    quote! {
        #state => {
            match __instruction {
                #instruction_pattern => { #body }
                _ => {}
            }
        }
    }
}

fn wrap_with_expression_matches(
    success_body: TokenStream,
    expr_vars: &[PatternVar],
) -> TokenStream {
    expr_vars.iter().rfold(success_body, |acc, pattern_var| {
        match pattern_var {
            PatternVar::Expression(details) => generate_expression_match(details, acc),
            PatternVar::Simple { .. } => acc, // Simple patterns don't need expression matching
        }
    })
}

fn generate_expression_match(
    details: &ExpressionVarDetails,
    success_body: TokenStream,
) -> TokenStream {
    let ExpressionVarDetails {
        sub_expr_var,
        pattern,
        nested_vars,
        ..
    } = details;

    if pattern.args.is_empty() {
        return success_body;
    }

    let nested_body = wrap_with_expression_matches(success_body, nested_vars);

    let vars_to_match: Vec<(usize, &PatternVar)> = pattern
        .args
        .iter()
        .enumerate()
        .filter_map(|(i, arg_obj)| {
            if PatternVar::needs_var(arg_obj) {
                Some(i)
            } else {
                None
            }
        })
        .zip(nested_vars.iter())
        .collect();

    vars_to_match
        .iter()
        .rfold(nested_body, |acc_body, &(i, pattern_var)| {
            let arg_pattern_tokens = generate_match_one_expression_pattern(pattern_var);

            let match_target_tokens = if pattern.args.len() == 1 {
                quote! { #sub_expr_var.clone() }
            } else {
                let syn_index = syn::Index::from(i);
                quote! { #sub_expr_var.#syn_index.clone() }
            };

            quote! {
                match #match_target_tokens {
                    #arg_pattern_tokens => { #acc_body }
                    _ => {}
                }
            }
        })
}

/// Generate a pattern for a single argument
fn generate_match_one_expression_pattern(pattern: &PatternVar) -> TokenStream {
    match pattern {
        PatternVar::Simple { expr_var, pattern } => {
            if is_variable_binding(pattern) {
                quote! { ref #expr_var }
            } else {
                quote! { #pattern }
            }
        }
        PatternVar::Expression(details) => generate_match_one_expression_pattern_argument(details),
    }
}

fn generate_match_one_expression_pattern_argument(
    ExpressionVarDetails {
        expr_var,
        sub_expr_var,
        pattern,
        ..
    }: &ExpressionVarDetails,
) -> TokenStream {
    let bind_expression = pattern.binding.as_ref().map(|_| quote! { ref #expr_var @ });
    let type_name = &pattern.type_name;

    quote! {
        #bind_expression ::bn_bdash_extras::llil::Expression {
            kind: ::bn_bdash_extras::llil::ExpressionKind::#type_name(ref #sub_expr_var),
            ..
        }
    }
}

fn build_instruction_pattern(
    pattern: &InstructionPattern,
    has_guard: bool,
) -> (TokenStream, Vec<PatternVar>) {
    let mut id_source = IdSource(1..);

    let (expr_vars, pattern_elements): (Vec<_>, Vec<_>) = pattern
        .args
        .iter()
        .map(|arg| match arg {
            ArgumentPattern::Simple(pat) => {
                if has_guard && is_variable_binding(pat) {
                    (None, quote! { ref #pat })
                } else {
                    (None, quote! { #pat })
                }
            }
            ArgumentPattern::Expression(pattern) => {
                let details = ExpressionVarDetails::new(pattern, &mut id_source);
                (
                    Some(PatternVar::Expression(details.clone())),
                    generate_match_one_expression_pattern_argument(&details),
                )
            }
        })
        .unzip();

    let instr_name = &pattern.name;
    let full_pattern = quote! {
        ::bn_bdash_extras::llil::InstructionKind::#instr_name(#(#pattern_elements),*)
    };

    (full_pattern, expr_vars.into_iter().flatten().collect())
}

/// Whether a pattern is a simple variable binding.
fn is_variable_binding(pat: &Pat) -> bool {
    match pat {
        Pat::Ident(ident_pat) => {
            // Simple identifiers without subpatterns are variable bindings
            ident_pat.by_ref.is_none()
                && ident_pat.mutability.is_none()
                && ident_pat.subpat.is_none()
        }
        _ => false,
    }
}

fn collect_all_let_bindings(pattern_vars: &[PatternVar]) -> Vec<TokenStream> {
    let mut bindings = Vec::new();

    for pattern_var in pattern_vars {
        collect_let_bindings_from_pattern_var(pattern_var, &mut bindings);
    }

    bindings
}

fn collect_let_bindings_from_pattern_var(
    pattern_var: &PatternVar,
    bindings: &mut Vec<TokenStream>,
) {
    match pattern_var {
        PatternVar::Simple { expr_var, pattern } if is_variable_binding(pattern) => {
            bindings.push(quote! {
                    let #pattern = ::bn_bdash_extras::llil::ExtractBoundValue::extract_bound_value(#expr_var.clone());
                });
        }
        PatternVar::Simple { .. } => {}
        PatternVar::Expression(ExpressionVarDetails {
            expr_var,
            pattern,
            nested_vars,
            ..
        }) => {
            if let Some(binding) = &pattern.binding {
                bindings.push(quote! {
                    let #binding = ::bn_bdash_extras::llil::ExtractBoundValue::extract_bound_value(#expr_var.clone());
                });
            }

            for nested_var in nested_vars {
                collect_let_bindings_from_pattern_var(nested_var, bindings);
            }
        }
    }
}

fn generate_final_match_cases(results: &[MatchResult], arms: &[MatchArm]) -> Vec<TokenStream> {
    results
        .iter()
        .zip(arms.iter())
        .map(|(result, arm)| {
            let MatchResult { arm_index, bound_vars, body } = result;
            let variant_name = format_ident!("Arm{}", arm_index);

            let bindings = bound_vars.iter().map(|var| &var.ident);

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
