use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Ident, Data, Fields, Type, PathArguments, Field, GenericArgument};
use syn::spanned::Spanned;
use syn::__private::{Span, TokenStream2};

fn get_optional_build_method(f: &Field) -> Option<&GenericArgument> {
    let ty = &f.ty;
    match ty {
        Type::Path(ref type_path) => {
            if type_path.path.segments.len() == 1 {
                let path_segment = &type_path.path.segments.first().unwrap();
                let ident = &path_segment.ident;
                if ident == "Option" {
                    match &path_segment.arguments {
                        PathArguments::AngleBracketed(args) => {
                            if args.args.len() == 1 {
                                let ty = args.args.first().unwrap();
                                Some(ty)
                            } else {
                                None
                            }
                        }
                        _ => { None }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => { None }
    }
}

fn get_build_method(data: &Data, name: &Ident) -> TokenStream2 {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let check_err = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let name_string = match name {
                            None => {"".into()}
                            Some(name) => {
                                format!("{}", name)
                            }
                        };
                        let is_option_arg = get_optional_build_method(&f);
                        if is_option_arg.is_none() {
                            quote_spanned! {f.span()=>
                                let #name = match self.#name {
                                    None => {
                                        let mut err_string = format!("{}", #name_string);
                                        err_string += " is unset!";
                                        let err_string: Box<dyn ::std::error::Error> = err_string.into();
                                        return Err(err_string);
                                    },
                                    Some(ref #name) => { #name.to_owned() }
                                };
                            }
                        } else {
                            quote_spanned! {f.span()=>
                                let #name = self.#name.clone();
                            }
                        }
                    });
                    let build = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! {f.span()=>
                            #name,
                        }
                    });
                    quote! {
                        pub fn build(&mut self) -> Result<#name, Box<dyn ::std::error::Error>> {
                            #(#check_err)*
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
                        match get_optional_build_method(&f)  {
                            Some(GenericArgument::Type(ty)) => {
                                quote_spanned! {f.span()=>
                                    fn #name(&mut self, #name: #ty) -> &mut Self {
                                        self.#name = Some(#name);
                                        self
                                    }
                                }}
                            _ => {
                                quote_spanned! {f.span()=>
                                    fn #name(&mut self, #name: #ty) -> &mut Self {
                                        self.#name = Some(#name);
                                        self
                                    }}
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
                        if get_optional_build_method(&f).is_none() {
                            quote! { #name: Option<#ty> }
                        } else {
                            quote! { #name: #ty }
                        }
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
            #build_method
        }
    };
    TokenStream::from(tokens)
}
