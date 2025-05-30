mod llil;

use proc_macro::TokenStream;

#[doc(hidden)]
#[proc_macro]
pub fn try_let_instr(input: TokenStream) -> TokenStream {
    llil::try_let_instr::try_let_instr(input)
}

#[doc(hidden)]
#[proc_macro_derive(InstrMatch, attributes(pattern))]
pub fn instr_match_derive(input: TokenStream) -> TokenStream {
    llil::derive_instr_match::instr_match_derive(input)
}

#[doc(hidden)]
#[proc_macro]
pub fn match_instr(input: TokenStream) -> TokenStream {
    llil::match_instr::match_instr(input).unwrap_or_else(|e| e.to_compile_error().into())
}
