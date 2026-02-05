//! Procedural macros for IRQL safety.
//!
//! This is an internal crate. Use the `irql` crate instead.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, GenericArgument, ImplItem, ItemFn, ItemImpl, PathArguments, Token, Type, parse::Parser,
    parse_quote, punctuated::Punctuated,
};

#[proc_macro]
pub fn call_irql_inner(input: TokenStream) -> TokenStream {
    match call_irql_inner_impl(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn call_irql_inner_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let parser = Punctuated::<Expr, Token![,]>::parse_separated_nonempty;
    let exprs = parser.parse(input)?;

    let exprs_vec: Vec<_> = exprs.into_iter().collect();

    if exprs_vec.len() != 2 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "call_irql_inner requires exactly 2 arguments (irql_level, expression), found {}",
                exprs_vec.len()
            ),
        ));
    }

    let irql_expr = &exprs_vec[0];
    let code_expr = exprs_vec[1].clone();

    let output = transform_call_expression(code_expr, irql_expr)?;

    Ok(output.into())
}

fn transform_call_expression(
    expr: Expr,
    irql_expr: &Expr,
) -> syn::Result<proc_macro2::TokenStream> {
    match expr {
        Expr::Call(mut call) => {
            if let Expr::Path(ref mut path_expr) = *call.func {
                if path_expr.path.segments.is_empty() {
                    return Err(syn::Error::new_spanned(path_expr, "function path is empty"));
                }

                let last_segment = path_expr.path.segments.last_mut().unwrap();

                let irql_type: Type = parse_quote! { ::irql::#irql_expr };
                let mut args: Punctuated<GenericArgument, Token![,]> = Punctuated::new();
                args.push(GenericArgument::Type(irql_type));

                last_segment.arguments = PathArguments::AngleBracketed(parse_quote! { ::<#args> });
            }
            Ok(quote! { #call })
        }
        Expr::MethodCall(mut method) => {
            let irql_type: Type = parse_quote! { ::irql::#irql_expr };
            let mut args: Punctuated<GenericArgument, Token![,]> = Punctuated::new();
            args.push(GenericArgument::Type(irql_type));

            method.turbofish = Some(parse_quote! { ::<#args> });
            Ok(quote! { #method })
        }
        _ => Err(syn::Error::new_spanned(
            &expr,
            "call_irql! expects a function call or method call expression",
        )),
    }
}

#[proc_macro_attribute]
pub fn requires_irql(attr: TokenStream, item: TokenStream) -> TokenStream {
    match requires_irql_impl(attr, item) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn requires_irql_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if attr.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected IRQL level argument, e.g., #[requires_irql(Passive)]",
        ));
    }

    let irql_type: Type = syn::parse(attr)?;

    if let Ok(mut input_fn) = syn::parse::<ItemFn>(item.clone()) {
        process_function(&mut input_fn, &irql_type);
        return Ok(TokenStream::from(quote! { #input_fn }));
    }

    if let Ok(mut input_impl) = syn::parse::<ItemImpl>(item) {
        process_impl_block(&mut input_impl, &irql_type);
        return Ok(TokenStream::from(quote! { #input_impl }));
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "#[requires_irql] can only be applied to functions or impl blocks",
    ))
}

fn process_function(input_fn: &mut ItemFn, irql_type: &Type) {
    input_fn.sig.generics.params.push(parse_quote! { IRQL });

    let where_clause = input_fn.sig.generics.make_where_clause();
    where_clause
        .predicates
        .push(parse_quote! { IRQL: ::irql::IrqlCanRaiseTo<#irql_type> });

    input_fn.block.stmts.insert(
        0,
        parse_quote! {
            #[allow(unused_macros)]
            macro_rules! call_irql {
                ($($tt:tt)*) => {
                    ::irql::call_irql_inner!(#irql_type, $($tt)*)
                };
            }
        },
    );

    input_fn.block.stmts.insert(
        0,
        parse_quote! {
            let _ = ::core::marker::PhantomData::<IRQL>;
        },
    );
}

fn process_impl_block(input_impl: &mut ItemImpl, irql_type: &Type) {
    for item in &mut input_impl.items {
        if let ImplItem::Fn(method) = item {
            method.sig.generics.params.push(parse_quote! { IRQL });

            let where_clause = method.sig.generics.make_where_clause();
            where_clause
                .predicates
                .push(parse_quote! { IRQL: ::irql::IrqlCanRaiseTo<#irql_type> });

            method.block.stmts.insert(
                0,
                parse_quote! {
                    #[allow(unused_macros)]
                    macro_rules! call_irql {
                        ($($tt:tt)*) => {
                            ::irql::call_irql_inner!(#irql_type, $($tt)*)
                        };
                    }
                },
            );

            method.block.stmts.insert(
                0,
                parse_quote! {
                    let _ = ::core::marker::PhantomData::<IRQL>;
                },
            );
        }
    }
}

#[proc_macro_attribute]
pub fn root_irql(attr: TokenStream, item: TokenStream) -> TokenStream {
    match root_irql_impl(attr, item) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

fn root_irql_impl(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if attr.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected IRQL level argument, e.g., #[root_irql(Passive)]",
        ));
    }

    let irql_type: Type = syn::parse(attr)?;
    let mut input_fn: ItemFn = syn::parse(item)?;

    input_fn.block.stmts.insert(
        0,
        parse_quote! {
            #[allow(unused_macros)]
            macro_rules! call_irql {
                ($($tt:tt)*) => {
                    ::irql::call_irql_inner!(#irql_type, $($tt)*)
                };
            }
        },
    );

    Ok(TokenStream::from(quote! { #input_fn }))
}
