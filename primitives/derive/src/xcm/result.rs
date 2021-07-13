//! XCM errors
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, Ident, Variant};

const XCM_ERRORS: [(&str, &str); 27] = [
    ("", "Undefined"),
    ("", "Overflow"),
    (
        "The operation is intentionally unsupported",
        "Unimplemented",
    ),
    ("", "UnhandledXcmVersion"),
    ("", "UnhandledXcmMessage"),
    ("", "UnhandledEffect"),
    ("", "EscalationOfPrivilege"),
    ("", "UntrustedReserveLocation"),
    ("", "UntrustedTeleportLocation"),
    ("", "DestinationBufferOverflow"),
    ("", "SendFailed"),
    ("", "CannotReachDestination"),
    ("", "MultiLocationFull"),
    ("", "FailedToDecode"),
    ("", "BadOrigin"),
    ("", "ExceedsMaxMessageSize"),
    ("", "FailedToTransactAsset"),
    ("", "WeightLimitReached"),
    ("", "Wildcard"),
    ("", "TooMuchWeightRequired"),
    ("", "NotHoldingFees"),
    ("", "WeightNotComputable"),
    ("", "Barrier"),
    ("", "NotWithdrawable"),
    ("", "LocationCannotHold"),
    ("", "TooExpensive"),
    ("", "AssetNotFound"),
];

/// Build enum variant
pub fn build_variant(unit: &str, _doc: String) -> Variant {
    Variant {
        attrs: vec![],
        ident: Ident::new(unit, Span::call_site()),
        fields: Fields::Unit,
        discriminant: None,
    }
}

/// Extends xcm errors
pub fn error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match input.data {
        Data::Enum(ref _data) => {}
        _ => return derive_error!("Expect enum"),
    }

    let expanded = quote! {
        #input
    };

    TokenStream::from(expanded)
}
