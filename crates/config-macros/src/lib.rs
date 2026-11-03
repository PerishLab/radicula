use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, parse_macro_input};

mod app;
mod parse;

#[proc_macro_derive(Cascade, attributes(cascade))]
pub fn cascade(input: TokenStream) -> TokenStream {
    parse::expand(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn path(name: &Ident) -> Result<proc_macro2::TokenStream, syn::Error> {
    let found = crate_name("radicula-config")
        .map_err(|error| syn::Error::new_spanned(name, error.to_string()))?;
    let name = match found {
        FoundCrate::Itself => format_ident!("radicula_config"),
        FoundCrate::Name(name) => format_ident!("{}", name),
    };
    Ok(quote!(::#name))
}
