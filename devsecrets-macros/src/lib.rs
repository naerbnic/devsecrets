extern crate proc_macro;
use proc_macro::TokenStream;

mod macro_impls {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use std::path::PathBuf;
    use syn::{Ident, Token};

    struct DevsecretsIdDecl {
        is_pub: bool,
        name: Ident,
    }

    impl syn::parse::Parse for DevsecretsIdDecl {
        fn parse(parser: syn::parse::ParseStream) -> syn::parse::Result<Self> {
            let first_tok = parser.lookahead1();
            let is_pub = first_tok.peek(Token![pub]);
            if is_pub {
                parser.parse::<Token![pub]>()?;
            }
            let name = parser.parse::<Ident>()?;

            Ok(DevsecretsIdDecl { is_pub, name })
        }
    }

    pub fn devsecrets_id_impl(input: TokenStream) -> TokenStream {
        let manifest_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should exist during compilation.")
            .into();
        let id = devsecrets_core::read_devsecrets_id(manifest_dir)
            .expect("Problem reading uuid file.")
            .expect("Uuid file does not exist");

        let id_decl = match syn::parse2::<DevsecretsIdDecl>(input) {
            Ok(decl) => decl,
            Err(e) => return e.to_compile_error(),
        };

        let pub_fragment = if id_decl.is_pub {
            quote!(pub)
        } else {
            quote!()
        };

        let name = &id_decl.name;
        let uuid_str = syn::LitStr::new(id.id_str(), Span::call_site());

        quote! {
            #pub_fragment static #name: ::devsecrets::Id =
                ::devsecrets::Id(::devsecrets::internal_core::DevSecretsId(
                    ::std::borrow::Cow::Borrowed(#uuid_str)));
        }
    }
}

#[proc_macro]
pub fn devsecrets_id(t: TokenStream) -> TokenStream {
    macro_impls::devsecrets_id_impl(t.into()).into()
}
