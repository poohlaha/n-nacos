/**!
  `Bean` 装配, 包括 `@component`、`@inject` 等注解
*/

use proc_macro::TokenStream;
use proc_macro_error::__export::proc_macro2::Ident;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::{Attribute, DeriveInput, Lit, Meta, parse_macro_input};

mod component;

mod inject;

fn find_attribute(input: &DeriveInput, name: &str) -> String {
    let attr = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident(name) {
                attr.parse_args::<Lit>().ok()
            } else {
                None
            }
        });

    if attr.is_none() {
        panic!("{}", format!("Expect an attribute `{name}` !"));
    }

    if let Some(Lit::Str(lit)) = attr {
        return lit.value();
    }

    panic!("{}", format!("Expect an attribute `{name}` value !"));
}

#[proc_macro_derive(Component, attributes(name))]
#[proc_macro_error]
pub fn component(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let name = &input.ident;

    let component_name = find_attribute(&input,"name");
    let gen = quote! {
         // 只在非测试情况下引入 bean_factory 包
        #[cfg(not(test))]
        ::bean_factory::submit! {
            ::bean_factory::bean::BeanInstance::init_with_name::<#name>(#component_name)
        }
    };

    gen.into()
}

/// 声明一个 `Bean`, 不装配, 可以指定名字 `name`,

#[proc_macro_attribute]
#[proc_macro_error]
pub fn bean(args: TokenStream, input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let name = &input.ident;

    let args_str = args.to_string();
    eprintln!("args: {}", args_str);

    let mut component_name = "";
    if !args_str.is_empty() {
        component_name = &args_str;
    }

    eprintln!("component_name: {}", component_name);
    let gen = quote! {

    };

    gen.into()
}

#[proc_macro_derive(Inject)]
#[proc_macro_error]
pub fn inject(input: TokenStream) -> TokenStream {
    let gen = quote! {

    };

    gen.into()
}

/// 声明一个 `Bean`，并自动装配
#[proc_macro_derive(Autowried)]
#[proc_macro_error]
pub fn autowried(input: TokenStream) -> TokenStream {
    let gen = quote! {

    };

    gen.into()
}

/// 根据 `name` 自动装配
#[proc_macro_derive(Resource, attributes(name))]
#[proc_macro_error]
pub fn resource(input: TokenStream) -> TokenStream {
    let gen = quote! {

    };

    gen.into()
}

