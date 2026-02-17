//! Procedural macros for compile-time IRQL safety.
//!
//! **Internal crate** — use [`irql`](https://docs.rs/irql) instead.

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ItemImpl};

mod apply;
mod call_irql;
mod constraints;

use constraints::IrqlConstraints;

/// Compile-time IRQL constraint macro.
///
/// See the [`irql` crate docs](https://docs.rs/irql) for usage.
#[proc_macro_attribute]
pub fn irql(attr: TokenStream, item: TokenStream) -> TokenStream {
    match irql_impl(attr, item) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn irql_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if attr.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("missing IRQL constraint.\n{}", constraints::USAGE),
        ));
    }

    let c = IrqlConstraints::parse(attr)?;

    // Fixed entry point — IRQL level is known, no generic parameter added.
    if let Some(ref at) = c.at {
        let mut f: ItemFn = syn::parse(item).map_err(|_| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[irql(at = ...)] can only be applied to functions",
            )
        })?;
        apply::inject_drop_bounds(&mut f.sig.generics, &f.sig.inputs, at);
        apply::inject_body(&mut f.block.stmts, at, false);
        return Ok(quote! { #f }.into());
    }

    // Generic IRQL — function or impl block.
    // syn::parse consumes the TokenStream, so clone once for the fallback.
    if let Ok(mut f) = syn::parse::<ItemFn>(item.clone()) {
        apply::apply_to_function(&mut f, &c);
        return Ok(quote! { #f }.into());
    }

    let mut imp: ItemImpl = syn::parse(item).map_err(|_| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[irql] can only be applied to functions or impl blocks",
        )
    })?;
    if imp.trait_.is_some() {
        apply::apply_to_trait_impl(&mut imp, &c)?;
    } else {
        apply::apply_to_impl_block(&mut imp, &c);
    }
    Ok(quote! { #imp }.into())
}

/// Hidden helper for `call_irql!` — injects IRQL as turbofish type argument.
#[doc(hidden)]
#[proc_macro]
pub fn call_irql_inner(input: TokenStream) -> TokenStream {
    match call_irql::call_irql_inner_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}
