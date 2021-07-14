//! PINT proc-macros
extern crate proc_macro;

// mod derive;
mod xcm;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn xcm_error(_attr: TokenStream, item: TokenStream) -> TokenStream {
    xcm::error(item)
}
