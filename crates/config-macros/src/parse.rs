use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Field, Fields, GenericArgument as Argument, Ident, LitStr,
    PathArguments as Arguments, Type,
};

struct Row {
    name: Ident,
    external: String,
    ty: Type,
    section: bool,
    arg: bool,
}

#[derive(Default)]
struct Mode {
    section: bool,
    strict: bool,
}

pub fn expand(item: DeriveInput) -> Result<TokenStream, syn::Error> {
    let root = crate::path(&item.ident)?;
    let serde = LitStr::new(&format!("{}::serde", root), item.ident.span());
    let rows = rows(&item)?;
    let mode = marked(&item.attrs)?;
    let section = mode.section;
    if section && rows.iter().any(|row| row.arg) {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "a section holds no args, args belong to the app struct",
        ));
    }
    let name = &item.ident;
    let vis = &item.vis;
    let partial = format_ident!("{}Partial", name);
    let held = rows.iter().map(|row| row.held(vis, &root));
    let reads = rows.iter().map(|row| row.read(&root));
    let merges = rows.iter().map(|row| row.merge(&root));
    let strict = mode
        .strict
        .then(|| quote! { #[serde(deny_unknown_fields)] });
    let mut out = quote! {
        #[derive(Debug, Default, #root::serde::Deserialize)]
        #[serde(crate = #serde, default)]
        #strict
        #vis struct #partial {
            #(#held,)*
        }

        impl #root::Cascade for #name {
            type Partial = #partial;
            fn lookup(
                prefix: &str,
                get: &dyn Fn(&str) -> ::core::option::Option<::std::string::String>,
            ) -> ::core::result::Result<#partial, #root::Error> {
                ::core::result::Result::Ok(#partial { #(#reads,)* })
            }
            fn merge(mut self, over: #partial) -> Self {
                #(#merges)*
                self
            }
        }
    };
    if !section {
        out.extend(crate::app::expand(&item, &partial, &root));
        out.extend(armed(&item, &partial, &rows));
    }
    Ok(out)
}

fn rows(item: &DeriveInput) -> Result<Vec<Row>, syn::Error> {
    let Data::Struct(body) = &item.data else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "Cascade derives on a named-field struct",
        ));
    };
    let Fields::Named(fields) = &body.fields else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "Cascade derives on a named-field struct",
        ));
    };
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "Cascade derives on a plain struct",
        ));
    }
    fields.named.iter().map(row).collect()
}

impl Row {
    fn held(&self, vis: &syn::Visibility, root: &TokenStream) -> TokenStream {
        let field = &self.name;
        let ty = &self.ty;
        let rename = (self.name != self.external).then(|| {
            let external = &self.external;
            quote! { #[serde(rename = #external)] }
        });
        if self.section {
            quote! { #[serde(default)] #rename #vis #field: <#ty as #root::Cascade>::Partial }
        } else {
            quote! { #rename #vis #field: ::core::option::Option<#ty> }
        }
    }

    fn read(&self, root: &TokenStream) -> TokenStream {
        let field = &self.name;
        let ty = &self.ty;
        let upper = self.external.to_uppercase();
        if self.section {
            return quote! {
                #field: <#ty as #root::Cascade>::lookup(
                    &::std::format!("{}_{}", prefix, #upper),
                    get,
                )?
            };
        }
        quote! {
            #field: {
                let key = ::std::format!("{}_{}", prefix, #upper);
                #[allow(unused_imports)]
                use #root::Opaque as _;
                (&#root::Sniff::<#ty>(::core::marker::PhantomData)).take(get(&key), &key)?
            }
        }
    }

    fn merge(&self, root: &TokenStream) -> TokenStream {
        let field = &self.name;
        let ty = &self.ty;
        if self.section {
            return quote! {
                self.#field = <#ty as #root::Cascade>::merge(self.#field, over.#field);
            };
        }
        quote! {
            if let ::core::option::Option::Some(value) = over.#field {
                self.#field = value;
            }
        }
    }

    fn arm(&self, vis: &syn::Visibility) -> TokenStream {
        let field = &self.name;
        let long = self.external.replace('_', "-");
        let ty = bare(&self.ty).unwrap_or(&self.ty);
        quote! { #[arg(long = #long)] #vis #field: Option<#ty> }
    }

    fn carry(&self) -> TokenStream {
        let field = &self.name;
        if bare(&self.ty).is_some() {
            return quote! { #field: self.#field.map(::core::option::Option::Some) };
        }
        quote! { #field: self.#field }
    }
}

fn armed(item: &DeriveInput, partial: &Ident, rows: &[Row]) -> TokenStream {
    let flagged: Vec<&Row> = rows.iter().filter(|row| row.arg).collect();
    if flagged.is_empty() {
        return TokenStream::new();
    }
    let vis = &item.vis;
    let title = format_ident!("{}Args", item.ident);
    let fields = flagged.iter().map(|row| row.arm(vis));
    let moves = flagged.iter().map(|row| row.carry());
    quote! {
        #[derive(::clap::Args, Clone, Debug)]
        #vis struct #title {
            #(#fields,)*
        }
        impl #title {
            #vis fn partial(self) -> #partial {
                #partial { #(#moves,)* ..::core::default::Default::default() }
            }
        }
    }
}

fn marked(attrs: &[Attribute]) -> Result<Mode, syn::Error> {
    let mut mode = Mode::default();
    for attr in attrs {
        if !attr.path().is_ident("cascade") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                mode.section = true;
                return Ok(());
            }
            if meta.path.is_ident("strict") {
                mode.strict = true;
                return Ok(());
            }
            Err(meta.error("use #[cascade(section, strict)] on the struct"))
        })?;
    }
    Ok(mode)
}

fn row(field: &Field) -> Result<Row, syn::Error> {
    let mut section = false;
    let mut arg = false;
    let mut external = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("cascade") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("section") {
                section = true;
                return Ok(());
            }
            if meta.path.is_ident("arg") {
                arg = true;
                return Ok(());
            }
            if meta.path.is_ident("name") {
                let value = meta.value()?.parse::<LitStr>()?.value();
                external = Some(
                    (!value.is_empty())
                        .then_some(value)
                        .ok_or_else(|| meta.error("cascade field name must not be empty"))?,
                );
                return Ok(());
            }
            Err(meta
                .error("use #[cascade(section)], #[cascade(arg)], or #[cascade(name = \"field\")]"))
        })?;
    }
    if section && arg {
        return Err(syn::Error::new_spanned(
            field,
            "a section field cannot take #[cascade(arg)]",
        ));
    }
    let name = field.ident.clone().expect("fields are named");
    Ok(Row {
        external: external.unwrap_or_else(|| name.to_string()),
        name,
        ty: field.ty.clone(),
        section,
        arg,
    })
}

fn bare(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident != "Option" {
        return None;
    }
    let Arguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let Argument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner)
}
