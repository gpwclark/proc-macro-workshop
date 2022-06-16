use proc_macro::TokenStream;
use std::error::Error;
use std::fmt;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Ident, Data, Fields, Type, PathArguments, Field, GenericArgument, Meta, NestedMeta, Lit};
use syn::spanned::Spanned;
use syn::__private::{Span, TokenStream2};

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

fn get_builder_attribute_name(f: &Field) -> Option<String> {
    if let Some(attr) = f.attrs.first() {
        if attr.path.segments.first().unwrap().ident == "builder" {
            match attr.parse_meta() {
                Ok(Meta::List(meta)) => {
                    for x in meta.nested {
                        match x {
                            NestedMeta::Meta(y) => {
                                match y {
                                    Meta::Path(_) => {}
                                    Meta::List(_) => {}
                                    Meta::NameValue(pair) => {
                                        let lit = pair.lit;
                                        match lit {
                                            Lit::Str(partial_name) => {
                                                return Some(partial_name.value());
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            NestedMeta::Lit(_) => {}
                        }
                    }
                },
                _ => {
                }
            }
        }
    }
    None
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
                        let fn_name = get_builder_attribute_name(&f);
                        if let Some(fn_name) = fn_name {
                            let item_name = "item_".to_string() + &fn_name;
                            let item_name = Ident::new(&item_name, Span::call_site());
                            let fn_name = Ident::new(&fn_name, Span::call_site());
                            match get_inner_type(&f, "Vec") {
                                Some(GenericArgument::Type(ty)) => {
                                    things.push(quote_spanned! {f.span()=>
                                        fn #fn_name(&mut self, #item_name: #ty) -> &mut Self {
                                            if let Some(ref mut #name) = self.#name {
                                                #name.push(#item_name);
                                            } else {
                                                self.#name = Some(vec![#item_name]);
                                            }
                                            self
                                        }});
                                }
                                _ => {
                                    things.push(quote_spanned! {f.span()=>
                                        fn #fn_name(&mut self, #item_name: #ty) -> &mut Self {
                                            if let Some(#item_name) = self.#name {
                                                self.name.push(#item_name);
                                            } else {
                                                self.#name = Some(vec![#item_name]);
                                            }
                                            self
                                        }});
                                }
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
                _ => {
                    Err(BuilderError::new("Not implemented for field type."))
                }
            }
        }
        _ => { 
            Err(BuilderError::new("Not implemented for type."))
        }
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
