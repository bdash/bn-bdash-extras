mod analyze;
mod generate;
mod parser;

use proc_macro::TokenStream;

pub(crate) fn match_instr(input: TokenStream) -> syn::Result<TokenStream> {
    let parser::MacroInput { expr, arms } = syn::parse(input)?;
    let analyzed_arms = analyze::analyze_arms(arms)?;
    let tokens = generate::generate_match_code(expr, analyzed_arms)?;
    Ok(tokens.into())
}
