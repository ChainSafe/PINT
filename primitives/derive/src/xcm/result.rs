//! XCM errors
use crate::xcm;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Arm, DeriveInput, Expr, ExprMatch};

fn expand_match(arms: Vec<Arm>) -> ExprMatch {
    ExprMatch {
        attrs: Default::default(),
        match_token: Default::default(),
        expr: Box::new(Expr::Verbatim(quote! {e})),
        brace_token: Default::default(),
        arms,
    }
}

/// Extends xcm errors
pub fn error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // construct expr match
    let xcm_match = expand_match(xcm::xcm::expand_errors());
    let outcome_match = expand_match(xcm::outcome::expand_errors());

    // get generics
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        use xcm::v0::Error as XcmError;
        use xcm_assets::Error as OutcomeError;

        #input

        impl #impl_generics From<XcmError> for #ident #ty_generics #where_clause {
            fn from(e: XcmError) -> Self {
                #xcm_match
            }
        }

        impl #impl_generics From<OutcomeError> for #ident #ty_generics #where_clause {
            fn from(e: OutcomeError) -> Self {
                #outcome_match
            }
        }
    };

    TokenStream::from(expanded)
}
