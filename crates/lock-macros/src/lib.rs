#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    Expr, ItemFn, MetaNameValue, Token, parse::Parse, parse::ParseStream, punctuated::Punctuated,
};

struct Arguments {
    gate: Expr,
    error: Expr,
}

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let items = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;
        let (mut gate, mut error) = (None, None);
        for item in items {
            let slot = if item.path.is_ident("gate") {
                &mut gate
            } else if item.path.is_ident("error") {
                &mut error
            } else {
                return Err(syn::Error::new_spanned(item.path, "expected gate or error"));
            };
            if slot.replace(item.value).is_some() {
                return Err(syn::Error::new_spanned(
                    item.path,
                    "duplicate guard argument",
                ));
            }
        }
        let missing = || input.error("guard requires gate = expression and error = handler");
        Ok(Self {
            gate: gate.ok_or_else(missing)?,
            error: error.ok_or_else(missing)?,
        })
    }
}

#[proc_macro_attribute]
pub fn guard(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = syn::parse_macro_input!(arguments as Arguments);
    let function = syn::parse_macro_input!(item as ItemFn);
    expand(arguments, function)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(arguments: Arguments, mut function: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if let Some(constness) = &function.sig.constness {
        return Err(syn::Error::new_spanned(
            constness,
            "guard cannot protect a const function",
        ));
    }
    let receiver = Ident::new("gate", Span::mixed_site());
    let output = Ident::new("output", Span::mixed_site());
    let failure = Ident::new("failure", Span::mixed_site());
    let Arguments { gate, error } = arguments;
    let body = function.block;
    let run = if function.sig.asyncness.is_some() {
        quote! { #receiver.asynchronous().guard(|| async #body).await }
    } else {
        quote! { #receiver.guard(|| #body) }
    };
    function.block = Box::new(syn::parse_quote!({
        let #receiver = (#gate).clone();
        match #run {
            ::core::result::Result::Ok(#output) => #output,
            ::core::result::Result::Err(#failure) => (#error)(#failure),
        }
    }));
    Ok(quote! { #function })
}
