//! Applying IRQL constraints to functions and impl blocks.
//!
//! Core transformation:
//! 1. Insert an `IRQL` generic parameter with `IrqlCanRaiseTo`/`IrqlCanLowerTo` bounds.
//! 2. Add `SafeToDropAt<Level>` bounds on by-value parameters.
//! 3. Inject a local `call_irql!` macro into function bodies.

use syn::{FnArg, GenericArgument, ImplItem, ItemFn, ItemImpl, PathArguments, Type, parse_quote};

use crate::constraints::IrqlConstraints;

/// Insert `IRQL` generic and add hierarchy where-clause bounds.
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

/// Add `T: SafeToDropAt<Level>` bounds for each by-value parameter.
///
/// References (`&T`, `&mut T`) are skipped — they don't own the value.
/// A bare `self` receiver gets a `Self: SafeToDropAt<Level>` bound.
pub(crate) fn inject_drop_bounds(
    generics: &mut syn::Generics,
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    level: &Type,
) {
    let wc = generics.make_where_clause();

    for arg in inputs {
        match arg {
            FnArg::Typed(pat_type) => {
                if !matches!(&*pat_type.ty, Type::Reference(_)) {
                    let ty = &pat_type.ty;
                    wc.predicates.push(parse_quote! {
                        #ty: ::irql::SafeToDropAt<#level>
                    });
                }
            }
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() {
                    wc.predicates.push(parse_quote! {
                        Self: ::irql::SafeToDropAt<#level>
                    });
                }
            }
        }
    }
}

/// Inject the local `call_irql!` macro (and optionally `PhantomData<IRQL>`).
///
/// `phantom` inserts `let _ = PhantomData::<IRQL>` to suppress "unused generic"
/// warnings. Trait methods don't need it because the trait definition uses `IRQL`.
pub(crate) fn inject_body(stmts: &mut Vec<syn::Stmt>, level: &Type, phantom: bool) {
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

/// Apply all three transformations to a single method/function.
fn transform_method(
    sig: &mut syn::Signature,
    block: &mut syn::Block,
    c: &IrqlConstraints,
    phantom: bool,
) {
    apply_irql_bounds(&mut sig.generics, c);
    inject_drop_bounds(&mut sig.generics, &sig.inputs, c.call_level());
    inject_body(&mut block.stmts, c.call_level(), phantom);
}

/// Apply constraints to a standalone function.
pub(crate) fn apply_to_function(f: &mut ItemFn, c: &IrqlConstraints) {
    transform_method(&mut f.sig, &mut f.block, c, true);
}

/// Apply constraints to an inherent impl block (each method gets its own `IRQL` generic).
pub(crate) fn apply_to_impl_block(imp: &mut ItemImpl, c: &IrqlConstraints) {
    for item in &mut imp.items {
        if let ImplItem::Fn(m) = item {
            transform_method(&mut m.sig, &mut m.block, c, true);
        }
    }
}

/// Apply constraints to a trait impl block (e.g. `impl IrqlFn<()> for T`).
///
/// Rewrites the trait's generic arguments to include the IRQL level,
/// then constrains each method.
pub(crate) fn apply_to_trait_impl(imp: &mut ItemImpl, c: &IrqlConstraints) -> syn::Result<()> {
    let (_, path, _) = imp
        .trait_
        .as_mut()
        .ok_or_else(|| syn::Error::new_spanned(&imp.self_ty, "expected a trait impl"))?;
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

    // Rewrite: IrqlFn<()> -> IrqlFn<Max, ()> or IrqlFn<Max, (), Min>
    let max = c.call_level();
    seg.arguments = if let Some(ref min) = c.min {
        PathArguments::AngleBracketed(parse_quote! { <#max, #args, #min> })
    } else {
        PathArguments::AngleBracketed(parse_quote! { <#max, #args> })
    };

    // No PhantomData: trait methods use IRQL through the trait's own generics.
    for item in &mut imp.items {
        if let ImplItem::Fn(m) = item {
            transform_method(&mut m.sig, &mut m.block, c, false);
        }
    }
    Ok(())
}
