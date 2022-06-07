use proc_macro::TokenStream;
use std::any::Any;
use std::error::Error;
use std::fmt;
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, DeriveInput, Ident, Data, Fields, Type, PathArguments, Field, GenericArgument, Meta, NestedMeta, PathSegment};
use syn::spanned::Spanned;
use syn::__private::{Span, TokenStream2};
use syn::punctuated::Punctuated;
use syn::token::{Colon2, Comma, Paren};

fn get_inner_type<'a>(f: &'a Field, type_name: &str) -> Option<&'a GenericArgument> {
    let ty = &f.ty;
    match ty {
        Type::Path(ref type_path) => {
            if type_path.path.segments.len() == 1 {
                //TODO fix calls to unwrap
                let path_segment = &type_path.path.segments.first().unwrap();
                let ident = &path_segment.ident;
                if ident == type_name {
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
                        let is_option_arg = get_inner_type(&f, "Option");
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
#[derive(Debug)]
struct BuilderError {
    details: String
}

impl BuilderError {
    fn new(msg: &str) -> BuilderError {
        BuilderError{details: msg.to_string()}
    }
}

impl fmt::Display for BuilderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for BuilderError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn get_builder_impl(data: &Data) -> Result<TokenStream2, BuilderError> {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut things = vec![];
                    for f in fields.named.iter() {
                        let name = &f.ident;
                        let ty = &f.ty;
                        if let Some(attr) = &f.attrs.first() {
                            if attr.path.segments.first().unwrap().ident == "builder" {
                                match attr.parse_meta() {
                                    Ok(Meta::List(meta)) => {
                                                let partial_name = &meta.path.segments.first().clone().unwrap();
                                                    //.clone().unwrap().ident;
                                                things.push(quote_spanned! {f.span()=>
                                                    /// #partial_name
                                                    fn #partial_name(&mut self, #partial_name: #ty) -> &mut Self {
                                                        if let Some(#name) = self.#name {
                                                            #name.push(#partial_name);
                                                        } else {
                                                            self.#name = Some(vec![#partial_name]);
                                                        }
                                                        self
                                                    }});
                                        let mut partial_name = &meta.path.segments[0].ident;
                                        match get_inner_type(&f, "Vec") {
                                            Some(GenericArgument::Type(inner_ty)) => {
                                                things.push(quote_spanned! {f.span()=>
                                                    fn #partial_name(&mut self, #partial_name: #inner_ty) -> &mut Self {
                                                        if let Some(#name) = self.#name {
                                                            #name.push(#partial_name);
                                                        } else {
                                                            self.#name = Some(vec![#partial_name]);
                                                        }
                                                        self
                                                    }});
                                            },
                                            _ => things.push(quote_spanned!{f.span()=> 
                                                fn #name(&mut self, #name: #ty) -> &mut Self {
                                                    let #name = "Type Vec must have type argument.";
                                                    self
                                                }
                                            }),
                                        }
                                    },
                                    _ => {
                                        things.push(quote_spanned!{f.span()=> 
                                            fn #name(&mut self, #name: #ty) -> &mut Self {
                                                let #name = "Non name value meta.";
                                                self
                                            }
                                        });
                                    }
                                }
                            } else {
                                things.push(quote_spanned!{f.span()=> 
                                        fn #name(&mut self, #name: #ty) -> &mut Self {
                                            let #name = "Can only parse builder attribute.";
                                            self
                                        }
                                    });
                            }
                        } else {
                            match get_inner_type(&f, "Option") {
                                Some(GenericArgument::Type(ty)) => {
                                    things.push(quote_spanned! {f.span()=>
                                    fn #name(&mut self, #name: #ty) -> &mut Self {
                                        self.#name = Some(#name);
                                        self
                                    }})
                                },
                                _ => {
                                    things.push(quote_spanned! {f.span()=>
                                    fn #name(&mut self, #name: #ty) -> &mut Self {
                                        self.#name = Some(#name);
                                        self
                                    }})
                                }
                            }
                        }
                    }
                    Ok(quote! {
                       #(#things)*
                   })
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
                        if get_inner_type(&f, "Option").is_none() {
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

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let empty_builder = get_empty_builder(&input.data);
    let builder_defn = get_builder_definition(&input.data);
    let builder_impl = get_builder_impl(&input.data).unwrap();
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
