mod llil;

use proc_macro::TokenStream;

#[proc_macro]
pub fn try_let_instr(input: TokenStream) -> TokenStream {
    llil::try_let_instr::try_let_instr(input)
}

#[proc_macro_derive(InstrMatch, attributes(pattern))]
pub fn instr_match_derive(input: TokenStream) -> TokenStream {
    llil::derive_instruction_matcher::instr_match_derive(input)
}
