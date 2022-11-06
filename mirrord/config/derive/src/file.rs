use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Ident, Visibility};

use crate::{field::FileStructField, flag::ConfigFlags};

#[derive(Debug)]
pub struct FileStruct {
    pub vis: Visibility,
    pub ident: Ident,
    pub fields: Vec<FileStructField>,
    pub source: Ident,
    pub derive: Vec<Ident>,
}

impl FileStruct {
    pub fn new(
        vis: Visibility,
        source: Ident,
        fields: Vec<FileStructField>,
        flags: ConfigFlags,
    ) -> Self {
        let ConfigFlags { map_to, derive, .. } = flags;

        let ident =
            map_to.unwrap_or_else(|| Ident::new(&format!("File{}", &source), Span::call_site()));

        FileStruct {
            vis,
            source,
            ident,
            fields,
            derive,
        }
    }
}

impl ToTokens for FileStruct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let FileStruct {
            ident,
            vis,
            fields,
            source,
            derive,
        } = &self;

        let field_definitions = fields.iter().map(|field| field.definition());
        let field_impl = fields.iter().map(|field| field.implmentation(&source));

        tokens.extend(quote! {
            #[derive(Debug, Clone, serde::Deserialize, #(#derive),*)]
            #[serde(deny_unknown_fields)]
            #vis struct #ident { #(#field_definitions),* }

            impl crate::config::MirrordConfig for #ident {
                type Generated = #source;

                fn generate_config(self) -> crate::config::Result<Self::Generated> {
                    Ok(#source {
                        #(#field_impl),*
                    })
                }
            }

            impl crate::config::FromMirrordConfig for #source {
                type Generator = #ident;
            }
        });
    }
}
