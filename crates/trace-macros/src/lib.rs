use proc_macro::TokenStream;
use syn::{ItemFn, LitStr, parse_macro_input};
mod trace;
#[proc_macro_attribute]
pub fn span(args: TokenStream, input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(args as LitStr);
    let item = parse_macro_input!(input as ItemFn);
    trace::expand(name, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
