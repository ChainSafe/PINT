//! PINT proc-macros
extern crate proc_macro;

// mod derive;
mod xcm;

use proc_macro::TokenStream;

/// `#[xcm_error]`
///
/// This macro is used for expand errors of xcm::v0::Error
#[proc_macro_attribute]
pub fn xcm_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    xcm::error(item)
}
