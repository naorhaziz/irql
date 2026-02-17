//! `call_irql_inner!` — turbofish IRQL injection.
//!
//! Rewrites `call_irql!(f(args))` → `f::<IRQL>(args)` and
//! `call_irql!(x.m(args))` → `x.m::<IRQL>(args)`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, GenericArgument, PathArguments, Token, Type, parse::Parser, parse_quote,
    punctuated::Punctuated,
};

/// Prepend `irql_type` to the generic arguments of the given turbofish.
fn prepend_irql(
    irql_type: Type,
    existing: Option<&syn::AngleBracketedGenericArguments>,
) -> Punctuated<GenericArgument, Token![,]> {
    let mut args: Punctuated<GenericArgument, Token![,]> = Punctuated::new();
    args.push(GenericArgument::Type(irql_type));
    if let Some(existing) = existing {
        args.extend(existing.args.iter().cloned());
    }
    args
}

/// Rewrite a function or method call to inject the IRQL level as the first
/// turbofish type argument.
pub(crate) fn call_irql_inner_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let mut exprs = Punctuated::<Expr, Token![,]>::parse_separated_nonempty.parse(input)?;

    if exprs.len() != 2 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("expected (level, expr), got {} arguments", exprs.len()),
        ));
    }

    // Safe: length was verified to be exactly 2 above.
    let call_expr = exprs.pop().expect("len == 2").into_value();
    let irql_expr = exprs.pop().expect("len == 2").into_value();
    let irql_type: Type = parse_quote! { ::irql::#irql_expr };

    match call_expr {
        Expr::Call(mut call) => {
            if let Expr::Path(ref mut p) = *call.func {
                let seg = p.path.segments.last_mut().expect("non-empty path");
                let existing = match &seg.arguments {
                    PathArguments::AngleBracketed(a) => Some(a),
                    _ => None,
                };
                let args = prepend_irql(irql_type, existing);
                seg.arguments = PathArguments::AngleBracketed(parse_quote! { ::<#args> });
            } else {
                return Err(syn::Error::new_spanned(
                    &call.func,
                    "call_irql! requires a named function — indirect calls \
                     (closures, function pointers) are not supported",
                ));
            }
            Ok(quote! { #call }.into())
        }
        Expr::MethodCall(mut mc) => {
            let existing = mc.turbofish.as_ref();
            let args = prepend_irql(irql_type, existing);
            mc.turbofish = Some(parse_quote! { ::<#args> });
            Ok(quote! { #mc }.into())
        }
        other => Err(syn::Error::new_spanned(
            &other,
            "call_irql! expects a function or method call",
        )),
    }
}
