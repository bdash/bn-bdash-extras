use syn::{Pat, visit::Visit};

pub mod derive_instruction_matcher;
pub mod try_let_instr;

#[derive(Default)]
struct PatIdentCollector {
    idents: Vec<syn::Ident>,
}

impl Visit<'_> for PatIdentCollector {
    fn visit_pat_ident(&mut self, i: &syn::PatIdent) {
        // Only collect the identifier if it's not part of an @ binding pattern
        // @ bindings are handled separately
        if i.by_ref.is_none() && i.mutability.is_none() && i.subpat.is_none() {
            self.idents.push(i.ident.clone());
        } else if let Some((_, ref subpat)) = i.subpat {
            // Handle @ bindings: collect the identifier and visit the subpattern
            self.idents.push(i.ident.clone());
            self.visit_pat(subpat);
        } else {
            self.idents.push(i.ident.clone());
        }
    }
}

/// Traverses the given pattern and collects all identifier bindings.
fn collect_pattern_idents(pat: &Pat) -> Vec<syn::Ident> {
    let mut ident_collector = PatIdentCollector::default();
    ident_collector.visit_pat(pat);
    ident_collector.idents
}
