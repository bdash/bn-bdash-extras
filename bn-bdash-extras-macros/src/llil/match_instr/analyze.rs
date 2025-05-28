use syn::{Expr, Ident, Pat, Token, parse::Result};

#[derive(Debug)]
pub struct MatchArm {
    pub pattern_kind: PatternKind,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    Instruction(InstructionPattern),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct InstructionPattern {
    pub name: Ident,
    pub args: Vec<ArgumentPattern>,
    pub binding: Option<Ident>,
}

#[derive(Debug, Clone)]
pub enum ArgumentPattern {
    Simple(Pat),
    Expression(ExpressionPattern),
}

#[derive(Debug, Clone)]
pub struct ExpressionPattern {
    pub type_name: Ident,
    pub args: Vec<Pat>,
    pub binding: Option<Ident>,
}

pub fn analyze_arms(arms: Vec<super::parser::MatchArm>) -> Result<Vec<MatchArm>> {
    if arms.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "match_instr! requires at least one match arm",
        ));
    }

    let mut expanded_arms = Vec::new();

    for arm in arms {
        if let Pat::Or(or_pat) = &arm.pattern {
            // Expand OR pattern into multiple arms
            let alternatives = expand_or_pattern(or_pat, &arm)?;
            expanded_arms.extend(alternatives);
        } else {
            // Regular pattern - analyze as before
            let pattern_kind = analyze_match_pattern(&arm.pattern)?;
            expanded_arms.push(MatchArm {
                pattern_kind,
                guard: arm.guard,
                body: arm.body,
            });
        }
    }

    // Validate that the last arm is a wildcard to ensure the match is exhaustive
    if let Some(last_arm) = expanded_arms.last() {
        if !matches!(last_arm.pattern_kind, PatternKind::Wildcard) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "non-exhaustive patterns: LLIL instruction patterns not covered
                = help: ensure that all possible cases are covered by adding a wildcard pattern: `_ => { ... }`
                = note: the matched value is of type `llil::InstructionKind`",
            ));
        }
    }

    Ok(expanded_arms)
}

/// Expands an OR pattern into multiple match arms with the same guard and body
/// This simplifies the handling of OR patterns by treating each alternative as a separate arm
fn expand_or_pattern(or_pat: &syn::PatOr, arm: &super::parser::MatchArm) -> Result<Vec<MatchArm>> {
    let mut expanded = Vec::new();

    // Validate that all alternatives bind the same variables
    validate_or_pattern_bindings(or_pat)?;

    for case in &or_pat.cases {
        let pattern_kind = analyze_match_pattern(case)?;
        expanded.push(MatchArm {
            pattern_kind,
            guard: arm.guard.clone(),
            body: arm.body.clone(),
        });
    }

    Ok(expanded)
}

/// Validates that all alternatives in an OR pattern bind the same set of variables
fn validate_or_pattern_bindings(or_pat: &syn::PatOr) -> Result<()> {
    if or_pat.cases.is_empty() {
        return Err(syn::Error::new_spanned(or_pat, "Empty OR pattern"));
    }

    // Collect bindings from the first alternative
    let mut first_bindings: Vec<String> = collect_pattern_bindings(&or_pat.cases[0])
        .iter()
        .map(|ident| ident.to_string())
        .collect();
    first_bindings.sort();

    // Check that all other alternatives have the same bindings
    for (i, case) in or_pat.cases.iter().enumerate().skip(1) {
        let mut case_bindings: Vec<String> = collect_pattern_bindings(case)
            .iter()
            .map(|ident| ident.to_string())
            .collect();
        case_bindings.sort();

        if first_bindings.len() != case_bindings.len() {
            return Err(syn::Error::new_spanned(
                case,
                format!(
                    "OR pattern alternative {} binds {} variables, but the first alternative binds {}",
                    i + 1,
                    case_bindings.len(),
                    first_bindings.len()
                ),
            ));
        }

        // Check that variable names match
        for (first_name, case_name) in first_bindings.iter().zip(case_bindings.iter()) {
            if first_name != case_name {
                return Err(syn::Error::new_spanned(
                    case,
                    format!(
                        "OR pattern alternative {} binds variable '{}', but the first alternative binds '{}'",
                        i + 1,
                        case_name,
                        first_name
                    ),
                ));
            }
        }
    }

    Ok(())
}

pub fn collect_pattern_bindings(pat: &Pat) -> Vec<Ident> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct BindingCollector {
        bindings: Vec<Ident>,
    }

    impl Visit<'_> for BindingCollector {
        fn visit_pat_ident(&mut self, node: &syn::PatIdent) {
            if node.by_ref.is_none() && node.mutability.is_none() && node.subpat.is_none() {
                self.bindings.push(node.ident.clone());
            } else if let Some((_, ref subpat)) = node.subpat {
                // Handle @ bindings: collect the identifier and visit the subpattern
                self.bindings.push(node.ident.clone());
                self.visit_pat(subpat);
            } else {
                self.bindings.push(node.ident.clone());
            }
        }
    }

    let mut collector = BindingCollector::default();
    collector.visit_pat(pat);
    collector.bindings
}

fn analyze_match_pattern(pat: &Pat) -> Result<PatternKind> {
    match pat {
        Pat::Wild(_) => Ok(PatternKind::Wildcard),

        Pat::Or(_) => Err(syn::Error::new_spanned(
            pat,
            "OR patterns should be expanded before individual pattern analysis",
        )),

        Pat::Ident(ident_pat) => {
            if let Some((_, subpat)) = &ident_pat.subpat {
                // Handle @ binding patterns like: binding @ InstrName(args)
                match analyze_match_pattern(subpat)? {
                    PatternKind::Instruction(instr_pattern) => {
                        Ok(PatternKind::Instruction(InstructionPattern {
                            binding: Some(ident_pat.ident.clone()),
                            ..instr_pattern
                        }))
                    }
                    _ => Err(syn::Error::new_spanned(pat, "Invalid @ binding pattern")),
                }
            } else {
                // Simple instruction name without arguments (e.g., "Nop")
                Ok(PatternKind::Instruction(InstructionPattern {
                    name: ident_pat.ident.clone(),
                    args: vec![],
                    binding: None,
                }))
            }
        }

        Pat::TupleStruct(tuple_struct) => Ok(PatternKind::Instruction(InstructionPattern {
            name: extract_ident_from_path(&tuple_struct.path)?,
            args: analyze_argument_patterns(&tuple_struct.elems)?,
            binding: None,
        })),

        _ => Err(syn::Error::new_spanned(
            pat,
            "Unsupported pattern type for instruction matching",
        )),
    }
}

pub fn analyze_argument_patterns(
    elems: &syn::punctuated::Punctuated<Pat, Token![,]>,
) -> Result<Vec<ArgumentPattern>> {
    elems
        .iter()
        .map(|pat| {
            if let Some(expr_pattern) = analyze_expression_pattern(pat)? {
                Ok(ArgumentPattern::Expression(expr_pattern))
            } else {
                Ok(ArgumentPattern::Simple(pat.clone()))
            }
        })
        .collect()
}

pub fn analyze_expression_pattern(pat: &Pat) -> Result<Option<ExpressionPattern>> {
    match pat {
        // Handle @ binding patterns like: target @ ConstPtr(address)
        Pat::Ident(ident_pat) if ident_pat.subpat.is_some() => {
            let binding = ident_pat.ident.clone();
            if let Some((_, subpat)) = &ident_pat.subpat {
                if let Pat::TupleStruct(nested) = subpat.as_ref() {
                    return Ok(Some(ExpressionPattern {
                        type_name: extract_ident_from_path(&nested.path)?,
                        args: nested.elems.iter().cloned().collect(),
                        binding: Some(binding),
                    }));
                }
            }
        }
        // Direct expression pattern without binding
        Pat::TupleStruct(nested) => {
            return Ok(Some(ExpressionPattern {
                type_name: extract_ident_from_path(&nested.path)?,
                args: nested.elems.iter().cloned().collect(),
                binding: None,
            }));
        }
        _ => {}
    }
    Ok(None)
}

fn extract_ident_from_path(path: &syn::Path) -> Result<Ident> {
    if path.segments.len() == 1 {
        Ok(path.segments[0].ident.clone())
    } else {
        Err(syn::Error::new_spanned(
            path,
            "Complex paths not supported in instruction patterns",
        ))
    }
}
