use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr};

pub fn expand(name: LitStr, item: ItemFn) -> syn::Result<TokenStream> {
    admit(&item)?;
    let attrs = &item.attrs;
    let vis = &item.vis;
    let sig = &item.sig;
    let block = &item.block;
    if sig.asyncness.is_some() {
        Ok(quote! {
            #(#attrs)* #vis #sig {
                ::radicula_trace::carry(#name, async move #block).await
            }
        })
    } else {
        Ok(quote! {
            #(#attrs)* #vis #sig {
                ::radicula_trace::run(#name, || #block)
            }
        })
    }
}

fn admit(item: &ItemFn) -> syn::Result<()> {
    let sig = &item.sig;
    let caller = item
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("track_caller"));
    let modifiers = [
        (sig.constness.is_some(), "const"),
        (sig.unsafety.is_some(), "unsafe"),
        (sig.abi.is_some(), "extern"),
        (sig.variadic.is_some(), "variadic"),
        (caller, "track_caller"),
    ];
    if let Some((_, modifier)) = modifiers.into_iter().find(|(present, _)| *present) {
        return Err(syn::Error::new_spanned(
            sig,
            format!("span does not support {modifier} functions"),
        ));
    }
    Ok(())
}
