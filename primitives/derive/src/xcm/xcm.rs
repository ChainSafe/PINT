//! XCM errors
use proc_macro2::Span;
use quote::quote;
use syn::{
    punctuated::Punctuated, token::Comma, Arm, Expr, Ident, Pat, PatTuple, PatTupleStruct, Path,
    PathArguments, PathSegment,
};

const XCM_ERRORS: [&str; 27] = [
    "Undefined",
    "Unimplemented",
    "Overflow",
    "UnhandledXcmVersion",
    "UnhandledXcmMessage",
    "UnhandledEffect",
    "EscalationOfPrivilege",
    "UntrustedReserveLocation",
    "UntrustedTeleportLocation",
    "DestinationBufferOverflow",
    "SendFailed(_)",
    "CannotReachDestination(_, _)",
    "MultiLocationFull",
    "FailedToDecode",
    "BadOrigin",
    "ExceedsMaxMessageSize",
    "FailedToTransactAsset(_)",
    "WeightLimitReached(_)",
    "Wildcard",
    "TooMuchWeightRequired",
    "NotHoldingFees",
    "WeightNotComputable",
    "Barrier",
    "NotWithdrawable",
    "LocationCannotHold",
    "TooExpensive",
    "AssetNotFound",
];

/// Expand xcm errors
pub fn expand_errors() -> Vec<Arm> {
    XCM_ERRORS
        .iter()
        .map(|i| {
            let ident = Ident::new(
                if let Some(idx) = i.find('(') {
                    &i[0..idx]
                } else {
                    i
                },
                Span::call_site(),
            );
            let (body, pat) = (ident.clone(), {
                let count = i.matches('_').count();
                if count == 0 {
                    Pat::Verbatim(quote! { XcmError::#ident })
                } else {
                    let mut elems = Punctuated::new();
                    for _ in 0..count {
                        elems.push(Pat::Verbatim(quote! { _ }));
                    }

                    let mut segments = Punctuated::new();
                    segments.push(PathSegment {
                        ident: ident.clone(),
                        arguments: PathArguments::None,
                    });

                    let ts = Pat::TupleStruct(PatTupleStruct {
                        attrs: Default::default(),
                        path: Path {
                            leading_colon: None,
                            segments,
                        },
                        pat: PatTuple {
                            attrs: Default::default(),
                            paren_token: Default::default(),
                            elems,
                        },
                    });

                    Pat::Verbatim(quote! { XcmError::#ts })
                }
            });

            Arm {
                attrs: Default::default(),
                pat,
                guard: None,
                fat_arrow_token: Default::default(),
                body: Box::new(Expr::Verbatim(quote! { Self::#body })),
                comma: Some(Comma::default()),
            }
        })
        .collect()
}
