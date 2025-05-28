use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, Pat, Stmt, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

use super::collect_pattern_idents;

struct TryLetInstrInput {
    _let_token: Option<Token![let]>,
    pat: Pat,
    _eq: Token![=],
    expr: Expr,
    _else_token: Token![else],
    fallback: syn::Block,
}

impl Parse for TryLetInstrInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(TryLetInstrInput {
            _let_token: input.parse().ok(),
            pat: Pat::parse_single(input)?,
            _eq: input.parse()?,
            expr: input.parse()?,
            _else_token: input.parse()?,
            fallback: input.parse()?,
        })
    }
}

pub fn try_let_instr(input: TokenStream) -> TokenStream {
    let TryLetInstrInput { pat, expr, fallback, .. } =
        parse_macro_input!(input as TryLetInstrInput);

    let idents = collect_pattern_idents(&pat);
    let tuple = quote! { ( #(#idents),* ) };

    // If the fallback block has a single statement, unwrap it to avoid unnecessary braces
    let fallback_expr = if fallback.stmts.len() == 1 {
        match &fallback.stmts[0] {
            Stmt::Expr(expr, None) => expr.clone(),
            _ => Expr::Block(syn::ExprBlock {
                attrs: vec![],
                label: None,
                block: fallback,
            }),
        }
    } else {
        Expr::Block(syn::ExprBlock {
            attrs: vec![],
            label: None,
            block: fallback,
        })
    };

    let expanded = quote! {
        let #tuple = ::bn_bdash_extras::llil::match_instr!(
            #expr,
            #pat => #tuple,
            _ => #fallback_expr
        );
    };
    expanded.into()
}
