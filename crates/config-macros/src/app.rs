use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

pub fn expand(item: &DeriveInput, partial: &Ident, root: &TokenStream) -> TokenStream {
    let name = &item.ident;
    let vis = &item.vis;
    quote! {
        impl #name {
            #vis fn resolve(
                file: ::core::option::Option<&::std::path::Path>,
                prefix: &str,
                get: &dyn Fn(&str) -> ::core::option::Option<::std::string::String>,
            ) -> ::core::result::Result<Self, #root::Error> {
                Self::layered(file, prefix, get, ::core::default::Default::default())
            }
            #vis fn layered(
                file: ::core::option::Option<&::std::path::Path>,
                prefix: &str,
                get: &dyn Fn(&str) -> ::core::option::Option<::std::string::String>,
                over: #partial,
            ) -> ::core::result::Result<Self, #root::Error> {
                let mut held = <Self as ::core::default::Default>::default();
                if let ::core::option::Option::Some(path) = file {
                    held = #root::Cascade::merge(
                        held,
                        #root::load::<#partial>(path)?,
                    );
                }
                let seen = <Self as #root::Cascade>::lookup(prefix, get)?;
                held = #root::Cascade::merge(held, seen);
                ::core::result::Result::Ok(#root::Cascade::merge(held, over))
            }
        }
    }
}
