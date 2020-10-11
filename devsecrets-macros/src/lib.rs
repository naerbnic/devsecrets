extern crate proc_macro;
use proc_macro::TokenStream;

mod macro_impls {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use std::path::PathBuf;

    struct DevsecretsIdDecl;

    impl syn::parse::Parse for DevsecretsIdDecl {
        fn parse(stream: syn::parse::ParseStream) -> syn::parse::Result<Self> {
            if !stream.is_empty() {
                Err(stream.error("devsecrets_id!() must have no arguments"))
            } else {
                Ok(DevsecretsIdDecl)
            }
        }
    }

    pub fn devsecrets_id_impl(input: TokenStream) -> syn::Result<TokenStream> {
        let _ = syn::parse2::<DevsecretsIdDecl>(input)?;

        let manifest_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| {
                syn::Error::new(
                    Span::call_site(),
                    "CARGO_MANIFEST_DIR should exist during compilation.",
                )
            })?
            .into();
        let id = devsecrets_core::read_devsecrets_id(manifest_dir)
            .map_err(|e| {
                syn::Error::new(
                    Span::call_site(),
                    format!("Problem reading uuid file: {}", e),
                )
            })?
            .ok_or_else(|| syn::Error::new(Span::call_site(), "Uuid file does not exist"))?;

        let uuid_str = syn::LitStr::new(id.id_str(), Span::call_site());

        Ok(quote! {
                ::devsecrets::Id(::devsecrets::internal_core::DevSecretsId(
                    ::std::borrow::Cow::Borrowed(#uuid_str)))
        })
    }
}

#[proc_macro]
pub fn devsecrets_id(t: TokenStream) -> TokenStream {
    macro_impls::devsecrets_id_impl(t.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
