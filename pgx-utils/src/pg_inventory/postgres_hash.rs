use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::Ident;

pub struct PostgresHash {
    pub name: Ident,
}

impl PostgresHash {
    pub fn new(name: Ident) -> Self {
        Self { name }
    }
}

impl ToTokens for PostgresHash {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let inv = quote! {
            pgx::inventory::submit! {
                use core::any::TypeId;
                crate::__pgx_internals::PgxPostgresHash(pgx_utils::pg_inventory::InventoryPostgresHash {
                    name: stringify!(#name),
                    file: file!(),
                    line: line!(),
                    full_path: core::any::type_name::<#name>(),
                    module_path: module_path!(),
                    id: TypeId::of::<#name>(),
                })
            }
        };
        tokens.append_all(inv);
    }
}


#[derive(Debug)]
pub struct InventoryPostgresHash {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub full_path: &'static str,
    pub module_path: &'static str,
    pub id: core::any::TypeId,
}