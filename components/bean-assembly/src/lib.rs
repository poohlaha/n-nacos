/**!
  `Bean` 装配, 包括 `@component`、`@inject` 等注解
*/

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};
mod component;
mod inject;
#[proc_macro_derive(Component)]
#[proc_macro_error]
pub fn component(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let name = &input.ident;
    let gen = quote! {
        ::bean_factory::submit! {
            ::bean_factory::bean::BeanInstance::init::<#name>()
        }
    };

    gen.into()
}