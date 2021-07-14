//! Outcome error
use proc_macro2::Span;
use quote::quote;
use syn::{token::Comma, Arm, Expr, Ident, Pat};

const OUTCOME_ERRORS: [&str; 4] = [
    "BadLocation",
    "InvalidDestination",
    "NoCrossChainTransfer",
    "NotCrossChainTransferableAsset",
];

// expand outcome errors
pub fn expand_errors() -> Vec<Arm> {
    OUTCOME_ERRORS
        .iter()
        .map(|i| {
            let ident = Ident::new(i, Span::call_site());
            Arm {
                attrs: Default::default(),
                pat: Pat::Verbatim(quote! { OutcomeError::#ident }),
                guard: None,
                fat_arrow_token: Default::default(),
                body: Box::new(Expr::Verbatim(quote! { Self::#ident })),
                comma: Some(Comma::default()),
            }
        })
        .collect()
}
