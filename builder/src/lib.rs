use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Ident};
use syn::__private::Span;

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let builder = format!("{}Builder", name);
    let builder_name = Ident::new(&builder, Span::call_site());
    let tokens = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }
        }
        
        pub struct #builder_name {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }
        
    };
    TokenStream::from(tokens)
}
