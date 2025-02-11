use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn slipway_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = syn::parse_macro_input!(item as syn::ItemFn);
    quote::quote! {
        #[test_log::test]
        #parsed_item
    }
    .into()
}

#[proc_macro_attribute]
pub fn slipway_test_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_item = syn::parse_macro_input!(item as syn::ItemFn);
    quote::quote! {
        #[test_log::test(tokio::test(flavor = "current_thread"))]
        #parsed_item
    }
    .into()
}

// #[proc_macro_attribute]
// pub fn slipway_test_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     // Parse the incoming function
//     let input = parse_macro_input!(item as ItemFn);
//     let fn_name = &input.sig.ident;
//     let fn_block = &input.block;
//     let fn_vis = &input.vis;
//     let fn_attrs = input.attrs;
//     let fn_sig = &input.sig;

//     // Generate a function that uses `#[tokio::test]` on a current-thread runtime,
//     // then sets up a LocalSet to spawn the original test body using spawn_local.
//     let expanded = quote! {
//         #[tokio::test(flavor = "current_thread")]
//         #(#fn_attrs)*
//         #fn_vis #fn_sig {
//             let local = tokio::task::LocalSet::new();
//             local.run_until(async {
//                 tokio::task::spawn_local(async #fn_block)
//                     .await
//                     .expect("spawn_local failed");
//             }).await
//         }
//     };

//     TokenStream::from(expanded)
// }
