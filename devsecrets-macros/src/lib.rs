extern crate proc_macro;
use proc_macro::TokenStream;

mod macro_impls {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use std::path::PathBuf;
    use syn::{Ident, Token};

    struct DevsecretsConfig {
        is_pub: bool,
        name: Ident,
    }

    impl syn::parse::Parse for DevsecretsConfig {
        fn parse(parser: syn::parse::ParseStream) -> syn::parse::Result<Self> {
            let first_tok = parser.lookahead1();
            let is_pub = first_tok.peek(Token![pub]);
            if is_pub {
                parser.parse::<Token![pub]>()?;
            }
            parser.parse::<Token![static]>()?;
            let name = parser.parse::<Ident>()?;
            parser.parse::<Token![;]>()?;

            Ok(DevsecretsConfig { is_pub, name })
        }
    }

    pub fn devsecrets_config_impl(input: TokenStream) -> TokenStream {
        let manifest_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR should exist during compilation.")
            .into();
        let uuid_file = manifest_dir.join(".devsecrets_uuid.txt");
        let uuid = std::fs::read_to_string(uuid_file).expect(".devsecrets_uuid.txt exists");

        let config = match syn::parse2::<DevsecretsConfig>(input) {
            Ok(config) => config,
            Err(e) => return e.to_compile_error(),
        };

        let pub_fragment = if config.is_pub { quote!(pub) } else { quote!() };

        let name = &config.name;
        let uuid_str = syn::LitStr::new(&uuid, Span::call_site());

        quote! {
            ::devsecrets::internal::lazy_static! {
                #pub_fragment static ref #name: ::devsecrets::DevSecrets = ::devsecrets::DevSecrets::from_uuid_str(#uuid_str);
            }
        }
    }
}

#[proc_macro]
pub fn devsecrets_config(t: TokenStream) -> TokenStream {
    macro_impls::devsecrets_config_impl(t.into()).into()
}
