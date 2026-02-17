//! Parsing `#[irql(...)]` attribute arguments.
//!
//! Supports three forms:
//! - `#[irql(at = Level)]` — fixed entry point, no generic added.
//! - `#[irql(max = Level)]` — callable from `Level` or below.
//! - `#[irql(min = A, max = B)]` — callable in the range \[A, B\].
//!
//! Level names and hierarchy order are **not** validated here — the compiler
//! catches invalid levels through `IrqlCanRaiseTo` / `IrqlCanLowerTo` trait
//! bounds emitted by the macro.

use proc_macro::TokenStream;
use syn::{Meta, Token, Type, parse::Parser, parse_quote, punctuated::Punctuated};

/// Usage hint appended to error messages.
pub(crate) const USAGE: &str = "\
    valid forms:\n  \
    #[irql(at = Passive)]           — fixed entry point\n  \
    #[irql(max = Dispatch)]         — callable from Dispatch or below\n  \
    #[irql(min = Apc, max = Dispatch)] — callable in [Apc, Dispatch]";

/// Parsed `#[irql(at/min/max = Level)]` constraints.
pub(crate) struct IrqlConstraints {
    pub min: Option<Type>,
    pub max: Option<Type>,
    pub at: Option<Type>,
}

impl IrqlConstraints {
    /// Parse the token stream inside `#[irql(...)]`.
    pub fn parse(attr: TokenStream) -> syn::Result<Self> {
        let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse(attr)?;

        let (mut min, mut max, mut at) = (None, None, None);

        for meta in &metas {
            let nv = match meta {
                Meta::NameValue(nv) => nv,
                Meta::Path(p) => {
                    let ident = p.get_ident().map(|i| i.to_string()).unwrap_or_default();
                    return Err(syn::Error::new_spanned(
                        p,
                        format!(
                            "expected `key = Level`, got `{ident}` without a value.\n\
                             Did you mean `max = {ident}` or `at = {ident}`?\n{USAGE}"
                        ),
                    ));
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        format!("expected `key = Level`.\n{USAGE}"),
                    ));
                }
            };

            let key = nv.path.get_ident().ok_or_else(|| {
                syn::Error::new_spanned(
                    &nv.path,
                    format!("expected `at`, `min`, or `max`.\n{USAGE}"),
                )
            })?;

            let ty: Type = match &nv.value {
                syn::Expr::Path(p) => {
                    let path = &p.path;
                    parse_quote! { #path }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "expected an IRQL level name",
                    ));
                }
            };

            let slot = match key.to_string().as_str() {
                "at" => &mut at,
                "min" => &mut min,
                "max" => &mut max,
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!(
                            "unknown parameter `{key}`. Expected `at`, `min`, or `max`.\n{USAGE}"
                        ),
                    ));
                }
            };

            if slot.is_some() {
                return Err(syn::Error::new_spanned(
                    key,
                    format!("`{key}` specified twice"),
                ));
            }
            *slot = Some(ty);
        }

        // `at` is mutually exclusive with `min`/`max`.
        if at.is_some() && (min.is_some() || max.is_some()) {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "`at` cannot be combined with `min` or `max`.\n\
                     `at = Level` means a fixed entry point. Use `max` (with optional `min`) \
                     for callable functions.\n{USAGE}"
                ),
            ));
        }

        // Either `at` or `max` must be set.
        if at.is_none() && max.is_none() {
            let hint = if min.is_some() {
                "`min` alone is not sufficient — `max` is required to define the IRQL ceiling \
                 that `call_irql!` relies on."
            } else {
                "`max` is required (unless using `at`). It defines the IRQL ceiling that \
                 `call_irql!` relies on."
            };
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("{hint}\n{USAGE}"),
            ));
        }

        Ok(Self { min, max, at })
    }

    /// The IRQL level used by `call_irql!` inside the annotated body.
    ///
    /// Returns `at` if set, otherwise `max`.
    pub fn call_level(&self) -> &Type {
        self.at
            .as_ref()
            .or(self.max.as_ref())
            .expect("BUG: either `at` or `max` must be set")
    }
}
