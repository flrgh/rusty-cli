extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ItemMod, parse_macro_input};

const fn truthy(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
        || s.eq_ignore_ascii_case("yes")
        || s.eq_ignore_ascii_case("1")
        || s.eq_ignore_ascii_case("on")
        || s.eq_ignore_ascii_case("enable")
        || s.eq_ignore_ascii_case("enabled")
}

const RESTY_ENABLED: bool = {
    if let Some(v) = option_env!("RUSTY_CLI_TEST_RESTY") {
        truthy(v)
    } else {
        false
    }
};

#[proc_macro_attribute]
pub fn bin_test(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    let function_identifier = input.sig.ident.clone();
    let real_func = quote::format_ident!("__{}", input.sig.ident);
    input.sig.ident = real_func.clone();

    if RESTY_ENABLED {
        quote!(
            mod #function_identifier {
                use super::*;
                use super::testlib;

                #input

                #[test]
                fn rusty() {
                    use testlib;
                    let bin = testlib::RUSTY;
                    assert!(bin.exists(), "rusty-cli ({}) not found", bin);
                    #real_func(bin);
                }

                #[test]
                fn resty() {
                    use testlib;
                    let bin = testlib::RESTY;
                    assert!(bin.exists(), "resty-cli ({}) not found", bin);
                    #real_func(bin);
                }
            }
        )
    } else {
        quote!(
            #input
            #[test]
            fn #function_identifier() {
                use testlib;
                let bin = testlib::RUSTY;
                assert!(bin.exists(), "rusty-cli ({}) not found", bin);
                #real_func(bin);
            }
        )
    }
    .into()
}

#[proc_macro_attribute]
pub fn integration(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemMod);
    quote!(
        mod integration {
            use super::*;

            #input
        }
    )
    .into()
}
