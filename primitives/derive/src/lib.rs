//! PINT proc-macros
extern crate proc_macro;

mod xcm;

use proc_macro::TokenStream;

#[proc_macro_derive(xcm)]
pub fn xcm_error(input: TokenStream) -> TokenStream {
    xcm::error(input)
}
