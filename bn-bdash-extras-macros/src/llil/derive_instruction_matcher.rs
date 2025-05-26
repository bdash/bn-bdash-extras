use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, GenericParam, Meta, Pat, parse_macro_input};

use super::collect_pattern_idents;

/// Extracts the Rust pattern from the #[pattern(...)] attribute on the struct.
/// Panics if the attribute is missing or invalid.
fn extract_pattern_attr(input: &syn::DeriveInput) -> syn::Pat {
    for attr in &input.attrs {
        if !attr.path().is_ident("pattern") {
            continue;
        }
        if let Meta::List(list) = &attr.meta {
            return syn::parse::Parser::parse2(Pat::parse_single, list.tokens.clone())
                .expect("Invalid Rust pattern in `pattern` attribute");
        } else {
            panic!("`pattern` attribute must be a list, e.g. #[pattern(...)]");
        }
    }
    panic!("`pattern` attribute is required");
}

/// Builds the generics list for the impl block.
/// Always includes 'func, M, F, and appends any extra generics from
/// the struct definition (excluding those three).
fn build_impl_generics(original_generics: &syn::Generics) -> proc_macro2::TokenStream {
    let mut extra_generics = Vec::new();
    for param in &original_generics.params {
        match param {
            GenericParam::Lifetime(lifetime) => {
                if lifetime.lifetime.ident != "func" {
                    extra_generics.push(quote! { #lifetime });
                }
            }
            GenericParam::Type(type_param) => {
                if type_param.ident != "M" && type_param.ident != "F" {
                    extra_generics.push(quote! { #type_param });
                }
            }
            GenericParam::Const(const_param) => {
                extra_generics.push(quote! { #const_param });
            }
        }
    }
    if extra_generics.is_empty() {
        quote! { 'func, M, F }
    } else {
        quote! { 'func, M, F, #(#extra_generics),* }
    }
}

/// Builds the full struct type reference for use in the impl block.
/// Substitutes 'func, M, F for any generics named 'func, M, F in the struct, and preserves any others.
/// Returns a token stream representing either `StructName` or `StructName<...>` as appropriate.
fn build_struct_type_ref(
    original_generics: &syn::Generics,
    struct_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let mut struct_type_args = Vec::new();
    for param in &original_generics.params {
        match param {
            GenericParam::Lifetime(lifetime) => {
                if lifetime.lifetime.ident == "func" {
                    struct_type_args.push(quote! { 'func });
                } else {
                    let lifetime_ident = &lifetime.lifetime;
                    struct_type_args.push(quote! { #lifetime_ident });
                }
            }
            GenericParam::Type(type_param) => {
                if type_param.ident == "M" {
                    struct_type_args.push(quote! { M });
                } else if type_param.ident == "F" {
                    struct_type_args.push(quote! { F });
                } else {
                    let type_ident = &type_param.ident;
                    struct_type_args.push(quote! { #type_ident });
                }
            }
            GenericParam::Const(const_param) => {
                let const_ident = &const_param.ident;
                struct_type_args.push(quote! { #const_ident });
            }
        }
    }
    if struct_type_args.is_empty() {
        quote! { #struct_name }
    } else {
        quote! { #struct_name < #(#struct_type_args),* > }
    }
}

/// Builds the where clause predicates for the impl block.
/// Always includes the required trait bounds for M, F, and the relevant Binary Ninja types.
/// Appends any user-supplied where predicates from the struct's generics.
fn build_impl_where_predicates(original_generics: &syn::Generics) -> Vec<proc_macro2::TokenStream> {
    let mut where_predicates = Vec::new();
    where_predicates.push(quote! {
        M: ::binaryninja::low_level_il::function::FunctionMutability,
        F: ::binaryninja::low_level_il::function::FunctionForm,
        ::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>:
            ::binaryninja::low_level_il::instruction::InstructionHandler<'func, M, F>,
        ::binaryninja::low_level_il::expression::LowLevelILExpression<
            'func, M, F, ::binaryninja::low_level_il::expression::ValueExpr
        >: ::binaryninja::low_level_il::expression::ExpressionHandler<'func, M, F>
    });
    if let Some(where_clause) = &original_generics.where_clause {
        for predicate in &where_clause.predicates {
            where_predicates.push(quote! { #predicate });
        }
    }
    where_predicates
}

pub fn instr_match_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract pattern from the #[pattern(...)] attribute
    let pattern = extract_pattern_attr(&input);
    let bindings = collect_pattern_idents(&pattern);

    let impl_generics = build_impl_generics(&input.generics);
    let struct_type = build_struct_type_ref(&input.generics, &input.ident);
    let where_predicates = build_impl_where_predicates(&input.generics);

    let expanded = quote! {
        impl<#impl_generics> ::core::convert::TryFrom<
            &::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>
        > for #struct_type
        where
            #(#where_predicates),*
        {
            type Error = ();
            fn try_from(instr: &::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>) -> Result<Self, Self::Error> {
                let instr = *instr;
                let ( #(#bindings),* ) = ::bn_bdash_extras::llil::macros::match_instr!(
                    instr,
                    #pattern => ( #(#bindings),* ),
                    _ => return Err(()),
                );
                Ok(Self { #(#bindings: #bindings.into()),* })
            }
        }
        impl<#impl_generics> ::core::convert::TryFrom<
            ::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>
        > for #struct_type
        where
            #(#where_predicates),*
        {
            type Error = ();
            fn try_from(instr: ::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>) -> Result<Self, Self::Error> {
                <#struct_type as ::core::convert::TryFrom<
                    &::binaryninja::low_level_il::instruction::LowLevelILInstruction<'func, M, F>
                >>::try_from(&instr)
            }
        }
    };

    TokenStream::from(expanded)
}
