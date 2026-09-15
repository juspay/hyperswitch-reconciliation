use proc_macro2::Span;
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Token};

pub fn non_enum_error() -> syn::Error {
    syn::Error::new(Span::call_site(), "This macro only supports enums.")
}

pub(super) fn get_metadata_inner<'a, T: Parse + Spanned>(
    ident: &str,
    attrs: impl IntoIterator<Item = &'a Attribute>,
) -> syn::Result<Vec<T>> {
    attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident(ident))
        .try_fold(Vec::new(), |mut vec, attr| {
            vec.extend(attr.parse_args_with(Punctuated::<T, Token![,]>::parse_terminated)?);
            Ok(vec)
        })
}

pub(super) fn syn_error(span: Span, message: &str) -> syn::Error {
    syn::Error::new(span, message)
}
