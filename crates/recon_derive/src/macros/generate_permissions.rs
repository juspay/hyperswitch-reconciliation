use std::{collections::HashSet, str::FromStr};

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use strum::IntoEnumIterator;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Ident, Token,
};

mod kw {
    syn::custom_keyword!(permissions);
    syn::custom_keyword!(scopes);
    syn::custom_keyword!(entities);
}

#[derive(Debug, strum::EnumString, strum::EnumIter, strum::Display)]
enum EntityType {
    Profile,
}

#[derive(Debug, strum::EnumString, strum::EnumIter, strum::Display)]
enum Scope {
    Read,
    Write,
}

fn possible_values<T>() -> String
where
    T: IntoEnumIterator + ToString,
{
    T::iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_validated_idents<T>(
    input: ParseStream<'_>,
    field: &str,
    parent_span: Span,
) -> syn::Result<Vec<Ident>>
where
    T: FromStr + IntoEnumIterator + ToString,
{
    let inner;
    bracketed!(inner in input);
    let idents: Punctuated<Ident, Token![,]> = Punctuated::parse_terminated(&inner)?;

    let validated = idents
        .into_iter()
        .map(|ident| {
            T::from_str(&ident.to_string())
                .map(|_| ident.clone())
                .map_err(|_| {
                    syn::Error::new(
                        ident.span(),
                        format!(
                            "Unknown {field} `{ident}`. Allowed: {}",
                            possible_values::<T>()
                        ),
                    )
                })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    if validated.is_empty() {
        return Err(syn::Error::new(
            parent_span,
            format!("`{field}` list must not be empty"),
        ));
    }

    Ok(validated)
}

struct PermissionDef {
    name: Ident,
    scopes: Vec<Ident>,
    entities: Vec<Ident>,
}

impl Parse for PermissionDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let body;
        braced!(body in input);

        body.parse::<kw::scopes>()?;
        body.parse::<Token![:]>()?;
        let scopes = parse_validated_idents::<Scope>(&body, "scope", name.span())?;

        body.parse::<Token![,]>()?;

        body.parse::<kw::entities>()?;
        body.parse::<Token![:]>()?;
        let entities = parse_validated_idents::<EntityType>(&body, "entity", name.span())?;

        // Trailing comma is optional.
        let _ = body.parse::<Token![,]>();

        Ok(Self {
            name,
            scopes,
            entities,
        })
    }
}

pub struct GeneratePermissionsInput {
    permissions: Vec<PermissionDef>,
}

impl Parse for GeneratePermissionsInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<kw::permissions>()?;
        input.parse::<Token![:]>()?;

        let list;
        bracketed!(list in input);
        let perms: Punctuated<PermissionDef, Token![,]> = Punctuated::parse_terminated(&list)?;
        let permissions: Vec<PermissionDef> = perms.into_iter().collect();

        if permissions.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "`permissions` list must not be empty",
            ));
        }

        Ok(Self { permissions })
    }
}

pub fn generate_permissions(input: GeneratePermissionsInput) -> TokenStream {
    let mut variants: Vec<Ident> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for perm in &input.permissions {
        for entity in &perm.entities {
            for scope in &perm.scopes {
                let variant = format_ident!("{entity}{}{scope}", perm.name);
                if seen.insert(variant.to_string()) {
                    variants.push(variant);
                }
            }
        }
    }

    // No `#[serde(rename_all)]` / `#[strum(serialize_all)]` is intentional:
    // hyperswitch's authorize_token expects the PascalCase variant name on the wire
    // (e.g. "ProfileReconIngestionWrite"). Default serde/strum output for a Rust
    // PascalCase variant is the variant name verbatim.
    quote! {
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
            ::core::cmp::Eq,
            ::core::cmp::PartialEq,
            ::core::hash::Hash,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::strum::Display,
            ::strum::EnumString,
        )]
        pub enum Permission {
            #(#variants,)*
        }
    }
}
