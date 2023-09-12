use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

#[proc_macro_derive(AutoDeref)]
pub fn auto_deref(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let gen = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().unwrap().ty;
                quote! {
                    impl std::ops::Deref for #name {
                        type Target = #field_type;

                        fn deref(&self) -> &Self::Target {
                            &self.0
                        }
                    }

                    impl std::ops::DerefMut for #name {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.0
                        }
                    }
                }
            }
            _ => quote! {
                compile_error!("AutoDeref can only be used with tuple structs with a single field");
            },
        },
        _ => quote! {
            compile_error!("AutoDeref can only be used with structs");
        },
    };

    gen.into()
}
