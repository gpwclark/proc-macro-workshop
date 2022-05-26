use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Ident, Data, Fields};
use syn::spanned::Spanned;
use syn::__private::{Span, TokenStream2};

fn get_build_method(data: &Data, name: &Ident) -> TokenStream2 {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let build = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! {f.span()=>
                            #name: match self.#name {
                                None => {
                                     return Err(Box::new("No".into()) )
                                },
                                Some(#name) => {
                                    #name
                                }
                            },
                        }
                    });
                    quote! {
                        pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
                            Ok(#name {
                                #(#build)*
                            })
                        }
                   }
                }
                Fields::Unnamed(_) => { unimplemented!() }
                Fields::Unit => { unimplemented!() }
            }
        }
        Data::Enum(_) => { unimplemented!()}
        Data::Union(_) => { unimplemented!()}
    }
}

fn get_builder_impl(data: &Data) -> TokenStream2 {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let impls = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote_spanned! {f.span()=>
                            fn #name(&mut self, #name: #ty) -> &mut Self {
                                self.#name = Some(#name);
                                self
                            }
                            
                        }
                    });
                    quote! {
                       #(#impls)*
                   }
                }
                Fields::Unnamed(_) => { unimplemented!() }
                Fields::Unit => { unimplemented!() }
            }
        }
        Data::Enum(_) => { unimplemented!()}
        Data::Union(_) => { unimplemented!()}
    }
}

fn get_builder_definition(data: &Data) -> TokenStream2 {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let defn = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote! { #name: Option<#ty> }
                    });
                    quote! {
                       #(#defn,)*
                   }
                }
                Fields::Unnamed(_) => { unimplemented!() }
                Fields::Unit => { unimplemented!() }
            }
        }
        Data::Enum(_) => { unimplemented!()}
        Data::Union(_) => { unimplemented!()}
    }
}

fn get_empty_builder(data: &Data) -> TokenStream2 {
   match *data {
       Data::Struct(ref data) => {
           match data.fields {
               Fields::Named(ref fields) => {
                   let names = fields.named.iter().map(|f| {
                       let name = &f.ident;
                       quote! { #name }
                   });
                   quote! {
                       #(#names: None,)*
                   }
               }
               Fields::Unnamed(_) => { unimplemented!() }
               Fields::Unit => { unimplemented!() }
           }
       }
       Data::Enum(_) => { unimplemented!()}
       Data::Union(_) => { unimplemented!()}
   }
}

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let empty_builder = get_empty_builder(&input.data);
    let builder_defn = get_builder_definition(&input.data);
    let builder_impl = get_builder_impl(&input.data);
    let build_method = get_build_method(&input.data, &name);
    let builder = format!("{}Builder", name);
    let builder_name = Ident::new(&builder, Span::call_site());
    let tokens = quote! {
        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #empty_builder
                }
            }
        }
        
        pub struct #builder_name {
            #builder_defn
        }
        
        impl #builder_name {
            #builder_impl
        }
        
        impl #builder_name {
            #build_method
        }
    };
    TokenStream::from(tokens)
}
