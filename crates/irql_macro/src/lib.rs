//! Procedural macros for compile-time IRQL safety.
//!
//! **Internal crate** — use [`irql`](https://docs.rs/irql) instead.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, GenericArgument, ImplItem, ItemFn, ItemImpl, PathArguments, Token, Type, parse::Parser,
    parse_quote, punctuated::Punctuated,
};

/// Usage hint appended to error messages.
const USAGE: &str = "examples:\n  \
    #[irql(at = Passive)]\n  \
    #[irql(max = Dispatch)]\n  \
    #[irql(min = Apc, max = Dispatch)]";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Inserts an `IRQL` generic parameter and adds where-clause bounds.
fn apply_irql_bounds(generics: &mut syn::Generics, c: &IrqlConstraints) {
    generics.params.insert(0, parse_quote! { IRQL });
    let wc = generics.make_where_clause();
    if let Some(ref max) = c.max {
        wc.predicates
            .push(parse_quote! { IRQL: ::irql::IrqlCanRaiseTo<#max> });
    }
    if let Some(ref min) = c.min {
        wc.predicates
            .push(parse_quote! { IRQL: ::irql::IrqlCanLowerTo<#min> });
    }
}

/// Injects the local `call_irql!` macro (and optionally `PhantomData<IRQL>`).
fn inject_body(stmts: &mut Vec<syn::Stmt>, level: &Type, phantom: bool) {
    stmts.insert(
        0,
        parse_quote! {
            #[allow(unused_macros)]
            macro_rules! call_irql {
                ($($tt:tt)*) => {
                    ::irql::call_irql_inner!(#level, $($tt)*)
                };
            }
        },
    );
    if phantom {
        stmts.insert(
            0,
            parse_quote! { let _ = ::core::marker::PhantomData::<IRQL>; },
        );
    }
}

// ---------------------------------------------------------------------------
// Applying constraints to items
// ---------------------------------------------------------------------------

/// Standalone function.
fn apply_to_function(f: &mut ItemFn, c: &IrqlConstraints) {
    apply_irql_bounds(&mut f.sig.generics, c);
    inject_body(&mut f.block.stmts, c.call_level(), true);
}

/// Inherent impl block — each method gets its own `IRQL` generic.
fn apply_to_impl_block(imp: &mut ItemImpl, c: &IrqlConstraints) {
    for item in &mut imp.items {
        if let ImplItem::Fn(m) = item {
            apply_irql_bounds(&mut m.sig.generics, c);
            inject_body(&mut m.block.stmts, c.call_level(), true);
        }
    }
}

/// Trait impl block (e.g. `impl IrqlFn<()> for T`).
///
/// Rewrites the trait's generic arguments and constrains each method.
fn apply_to_trait_impl(imp: &mut ItemImpl, c: &IrqlConstraints) -> syn::Result<()> {
    let (_, path, _) = imp.trait_.as_mut().unwrap();
    let seg = path
        .segments
        .last_mut()
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "empty trait path"))?;
    let name = seg.ident.to_string();

    // Extract the user-written Args type (the only generic the user provides).
    let args = match &seg.arguments {
        PathArguments::AngleBracketed(a) if a.args.len() == 1 => match &a.args[0] {
            GenericArgument::Type(ty) => ty.clone(),
            other => return Err(syn::Error::new_spanned(other, "expected a type for Args")),
        },
        PathArguments::AngleBracketed(a) => {
            return Err(syn::Error::new_spanned(
                a,
                format!(
                    "expected 1 type parameter (Args), found {}; \
                     IRQL level is added automatically",
                    a.args.len()
                ),
            ));
        }
        _ => {
            return Err(syn::Error::new_spanned(
                seg,
                format!("{name} requires an Args type, e.g. {name}<()>"),
            ));
        }
    };

    // Rewrite: IrqlFn<()> → IrqlFn<Max, ()> or IrqlFn<Max, (), Min>
    let max = c.call_level();
    seg.arguments = if let Some(ref min) = c.min {
        PathArguments::AngleBracketed(parse_quote! { <#max, #args, #min> })
    } else {
        PathArguments::AngleBracketed(parse_quote! { <#max, #args> })
    };

    // Each method: bounds + call_irql! (no PhantomData for trait methods).
    let level = c.call_level();
    for item in &mut imp.items {
        if let ImplItem::Fn(m) = item {
            apply_irql_bounds(&mut m.sig.generics, c);
            inject_body(&mut m.block.stmts, level, false);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Parsed constraints from #[irql(at/min/max = Level)]
// ---------------------------------------------------------------------------

struct IrqlConstraints {
    min: Option<Type>,
    max: Option<Type>,
    at: Option<Type>,
}

impl IrqlConstraints {
    fn parse(attr: TokenStream) -> syn::Result<Self> {
        let metas = Punctuated::<syn::Meta, Token![,]>::parse_terminated.parse(attr)?;

        let (mut min, mut max, mut at) = (None, None, None);

        for meta in &metas {
            let nv = match meta {
                syn::Meta::NameValue(nv) => nv,
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        format!("expected `key = Level`.\n{USAGE}"),
                    ));
                }
            };

            let key = nv.path.get_ident().ok_or_else(|| {
                syn::Error::new_spanned(&nv.path, "expected `at`, `min`, or `max`")
            })?;

            let ty: Type = match &nv.value {
                syn::Expr::Path(p) => {
                    let path = &p.path;
                    parse_quote! { #path }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "expected an IRQL level (e.g. Passive, Dispatch)",
                    ));
                }
            };

            let slot = if key == "at" {
                &mut at
            } else if key == "min" {
                &mut min
            } else if key == "max" {
                &mut max
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("unknown parameter `{key}`.\n{USAGE}"),
                ));
            };

            if slot.is_some() {
                return Err(syn::Error::new_spanned(key, format!("duplicate `{key}`")));
            }
            *slot = Some(ty);
        }

        if at.is_some() && (min.is_some() || max.is_some()) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("`at` cannot be combined with `min` or `max`.\n{USAGE}"),
            ));
        }
        if at.is_none() && max.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "`max` is required (unless using `at`). \
                     It defines the IRQL ceiling that `call_irql!` relies on.\n{USAGE}"
                ),
            ));
        }

        Ok(Self { min, max, at })
    }

    /// The IRQL level used by `call_irql!` inside the annotated body.
    fn call_level(&self) -> &Type {
        self.at
            .as_ref()
            .or(self.max.as_ref())
            .expect("BUG: either `at` or `max` must be set")
    }
}

// ---------------------------------------------------------------------------
// #[irql(...)]
// ---------------------------------------------------------------------------

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
            format!("missing IRQL constraint.\n{USAGE}"),
        ));
    }

    let c = IrqlConstraints::parse(attr)?;

    // Fixed entry point — inject call_irql! with no generic.
    if let Some(ref at) = c.at {
        let mut f: ItemFn = syn::parse(item).map_err(|_| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[irql(at = ...)] can only be applied to functions",
            )
        })?;
        inject_body(&mut f.block.stmts, at, false);
        return Ok(quote! { #f }.into());
    }

    // Generic IRQL — function or impl block.
    if let Ok(mut f) = syn::parse::<ItemFn>(item.clone()) {
        apply_to_function(&mut f, &c);
        return Ok(quote! { #f }.into());
    }

    if let Ok(mut imp) = syn::parse::<ItemImpl>(item) {
        if imp.trait_.is_some() {
            apply_to_trait_impl(&mut imp, &c)?;
        } else {
            apply_to_impl_block(&mut imp, &c);
        }
        return Ok(quote! { #imp }.into());
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "#[irql] can only be applied to functions or impl blocks",
    ))
}

// ---------------------------------------------------------------------------
// call_irql_inner!  (plumbing used by the local call_irql! macro)
// ---------------------------------------------------------------------------

#[doc(hidden)]
#[proc_macro]
pub fn call_irql_inner(input: TokenStream) -> TokenStream {
    match call_irql_inner_impl(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

fn call_irql_inner_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let exprs: Vec<Expr> = Punctuated::<Expr, Token![,]>::parse_separated_nonempty
        .parse(input)?
        .into_iter()
        .collect();

    if exprs.len() != 2 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("expected (level, expr), got {} arguments", exprs.len()),
        ));
    }

    let irql_expr = &exprs[0];
    let irql_type: Type = parse_quote! { ::irql::#irql_expr };

    match exprs[1].clone() {
        Expr::Call(mut call) => {
            if let Expr::Path(ref mut p) = *call.func {
                if p.path.segments.is_empty() {
                    return Err(syn::Error::new_spanned(&*p, "empty function path"));
                }
                let seg = p.path.segments.last_mut().unwrap();
                let mut args: Punctuated<GenericArgument, Token![,]> = Punctuated::new();
                args.push(GenericArgument::Type(irql_type));
                if let PathArguments::AngleBracketed(ref existing) = seg.arguments {
                    args.extend(existing.args.iter().cloned());
                }
                seg.arguments = PathArguments::AngleBracketed(parse_quote! { ::<#args> });
            }
            Ok(quote! { #call }.into())
        }
        Expr::MethodCall(mut mc) => {
            let mut args: Punctuated<GenericArgument, Token![,]> = Punctuated::new();
            args.push(GenericArgument::Type(irql_type));
            if let Some(ref existing) = mc.turbofish {
                args.extend(existing.args.iter().cloned());
            }
            mc.turbofish = Some(parse_quote! { ::<#args> });
            Ok(quote! { #mc }.into())
        }
        other => Err(syn::Error::new_spanned(
            &other,
            "call_irql! expects a function or method call",
        )),
    }
}
