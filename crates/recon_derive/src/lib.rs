use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned, Attribute, Data, Token};
mod macros;
mod diesel_keyword {
    use syn::custom_keyword;

    custom_keyword!(storage_type);
    custom_keyword!(db_enum);
    custom_keyword!(text);
}

#[derive(Debug)]
enum DieselEnumMeta {
    StorageTypeEnum {
        keyword: diesel_keyword::storage_type,
        value: StorageType,
    },
}

impl DieselEnumMeta {
    pub fn get_storage_type(&self) -> &StorageType {
        match self {
            Self::StorageTypeEnum { value, .. } => value,
        }
    }
}

impl Parse for DieselEnumMeta {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(diesel_keyword::storage_type) {
            let keyword = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value = input.parse()?;
            Ok(Self::StorageTypeEnum { keyword, value })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for DieselEnumMeta {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::StorageTypeEnum { keyword, .. } => keyword.to_tokens(tokens),
        }
    }
}

impl Parse for StorageType {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let text = input.parse::<syn::LitStr>()?;
        let value = text.value();

        value.as_str().parse().map_err(|_| {
            syn::Error::new_spanned(
                &text,
                format!(
                    "Unexpected value for storage_type: `{value}`. Possible values are `{}`",
                    get_possible_values_for_enum::<Self>()
                ),
            )
        })
    }
}

/// Get all the variants of a enum in the form of a string
fn get_possible_values_for_enum<T>() -> String
where
    T: strum::IntoEnumIterator + ToString,
{
    T::iter()
        .map(|variants| variants.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
#[derive(Debug, strum::EnumString, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
enum StorageType {
    /// Store the Enum as Text value in the database
    Text,
    /// Store the Enum as Enum in the database. This requires a corresponding enum to be created
    /// in the database with the same name
    DbEnum,
}

/// Based on the storage type, derive appropriate diesel traits
/// This will add the appropriate #[diesel(sql_type)]
/// Since the `FromSql` and `ToSql` have to be derived for all the enums, this will add the
/// `DieselEnum` derive trait.
pub(crate) fn diesel_enum_attribute_macro(
    diesel_enum_meta: DieselEnumMeta,
    item: &syn::ItemEnum,
) -> syn::Result<TokenStream> {
    let diesel_derives =
        quote!(#[derive(diesel::AsExpression, diesel::FromSqlRow, recon_derive::DieselEnum) ]);

    match diesel_enum_meta {
        DieselEnumMeta::StorageTypeEnum {
            value: storage_type,
            ..
        } => match storage_type {
            StorageType::Text => Ok(quote! {
                #diesel_derives
                #[diesel(sql_type = ::diesel::sql_types::Text)]
                #[storage_type(storage_type = "text")]
                #item
            }),
            StorageType::DbEnum => {
                let name = &item.ident;
                let type_name = format_ident!("Db{name}");
                Ok(quote! {
                    #diesel_derives
                    #[diesel(sql_type = #type_name)]
                    #[storage_type(storage_type= "db_enum")]
                    #item
                })
            }
        },
    }
}

/// Derives the boilerplate code required for using an enum with `diesel` and a PostgreSQL database.
///
/// Storage Type can either be "text" or "db_enum"
/// Choosing text will store the enum as text in the database, whereas db_enum will map it to the
/// corresponding database enum
///
/// Works in tandem with the [`DieselEnum`][DieselEnum] derive macro to achieve the desired results.
/// The enum is required to implement (or derive) the [`ToString`][ToString] and the
/// [`FromStr`][FromStr] traits for the [`DieselEnum`][DieselEnum] derive macro to be used.
///
/// [DieselEnum]: crate::DieselEnum
/// [FromStr]: ::core::str::FromStr
/// [ToString]: ::std::string::ToString
///
/// # Example
///
/// ```ignore
/// use recon_derive::{diesel_enum};
///
/// // Deriving `FromStr` and `ToString` using the `strum` crate, you can also implement it
/// // yourself if required. (Required by the DieselEnum derive macro.)
/// #[derive(strum::Display, strum::EnumString)]
/// #[derive(Debug)]
/// #[diesel_enum(storage_type = "text")]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ````
#[proc_macro_attribute]
pub fn diesel_enum(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args_parsed = syn::parse_macro_input!(args as DieselEnumMeta);
    let item = syn::parse_macro_input!(item as syn::ItemEnum);

    diesel_enum_attribute_macro(args_parsed, &item)
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// Derives the boilerplate code required for using an enum with `diesel` and a PostgreSQL database.
/// The enum is required to implement (or derive) the [`ToString`][ToString] and the
/// [`FromStr`][FromStr] traits for this derive macro to be used.
///
/// Works in tandem with the [`diesel_enum`][diesel_enum] attribute macro to achieve the desired
/// results.
///
/// [diesel_enum]: macro@crate::diesel_enum
/// [FromStr]: ::core::str::FromStr
/// [ToString]: ::std::string::ToString
///
/// # Example
///
/// ```ignore
/// use recon_derive::diesel_enum;
///
/// // Deriving `FromStr` and `ToString` using the `strum` crate, you can also implement it
/// // yourself if required.
/// #[derive(strum::Display, strum::EnumString)]
/// #[derive(Debug)]
/// #[diesel_enum(storage_type = "db_enum")]
/// enum Color {
///     Red,
///     Green,
///     Blue,
/// }
/// ```
#[proc_macro_derive(DieselEnum, attributes(storage_type))]
pub fn diesel_enum_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let tokens = diesel_enum_derive_inner(&ast).unwrap_or_else(|error| error.to_compile_error());
    tokens.into()
}

trait DieselDeriveInputExt {
    /// Get all the error metadata associated with an enum.
    fn get_metadata(&self) -> syn::Result<Vec<DieselEnumMeta>>;
}

impl DieselDeriveInputExt for syn::DeriveInput {
    fn get_metadata(&self) -> syn::Result<Vec<DieselEnumMeta>> {
        get_metadata_inner("storage_type", &self.attrs)
    }
}

fn get_metadata_inner<'a, T: Parse + Spanned>(
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

pub(crate) fn diesel_enum_derive_inner(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let storage_type = ast.get_metadata()?;

    match storage_type
        .first()
        .ok_or(syn::Error::new(
            Span::call_site(),
            "Storage type must be specified",
        ))?
        .get_storage_type()
    {
        StorageType::Text => diesel_enum_text_derive_inner(ast),
        StorageType::DbEnum => diesel_enum_db_enum_derive_inner(ast),
    }
}

pub(crate) fn diesel_enum_text_derive_inner(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    match &ast.data {
        Data::Enum(_) => (),
        _ => return Err(non_enum_error()),
    };

    Ok(quote! {

        #[automatically_derived]
        impl #impl_generics ::diesel::serialize::ToSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.to_string().as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::diesel::deserialize::FromSql<::diesel::sql_types::Text, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn from_sql(value: ::diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
                use ::core::str::FromStr;

                Self::from_str(::core::str::from_utf8(value.as_bytes())?)
                    .map_err(|_| "Unrecognized enum variant".into())
            }
        }
    })
}

pub(crate) fn diesel_enum_db_enum_derive_inner(ast: &syn::DeriveInput) -> syn::Result<TokenStream> {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    match &ast.data {
        Data::Enum(_) => (),
        _ => return Err(non_enum_error()),
    };

    let struct_name = format_ident!("Db{name}");
    let type_name = format!("{name}");

    Ok(quote! {

        #[derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug, ::diesel::QueryId, ::diesel::SqlType)]
        #[diesel(postgres_type(name = #type_name))]
        pub struct #struct_name;

        #[automatically_derived]
        impl #impl_generics ::diesel::serialize::ToSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                use ::std::io::Write;

                out.write_all(self.to_string().as_bytes())?;
                Ok(::diesel::serialize::IsNull::No)
            }
        }

        #[automatically_derived]
        impl #impl_generics ::diesel::deserialize::FromSql<#struct_name, ::diesel::pg::Pg> for #name #ty_generics
        #where_clause
        {
            fn from_sql(value: ::diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
                use ::core::str::FromStr;

                Self::from_str(::core::str::from_utf8(value.as_bytes())?)
                    .map_err(|_| "Unrecognized enum variant".into())
            }
        }
    })
}

fn non_enum_error() -> syn::Error {
    syn::Error::new(Span::call_site(), "This macro only supports enums.")
}

/// Generates the function to get the value out of enum variant
/// Usage
/// ```
/// use recon_derive::TryGetEnumVariant;
///
/// impl std::fmt::Display for RedisError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
///         match self {
///             Self::UnknownResult => write!(f, "Unknown result")
///         }
///     }
/// }
///
/// impl std::error::Error for RedisError {}
///
/// #[derive(Debug)]
/// enum RedisError {
///     UnknownResult
/// }
///
/// #[derive(TryGetEnumVariant)]
/// #[error(RedisError::UnknownResult)]
/// enum RedisResult {
///     Set(String),
///     Get(i32)
/// }
/// ```
///
/// This will generate the function to get `String` and `i32` out of the variants
///
/// ```rust, ignore
/// impl RedisResult {
///     fn try_into_get(&self)-> Result<i32, RedisError> {
///         match self {
///             Self::Get(a) => Ok(a),
///             _=>Err(RedisError::UnknownResult)
///         }
///     }
///
///     fn try_into_set(&self)-> Result<String, RedisError> {
///         match self {
///             Self::Set(a) => Ok(a),
///             _=> Err(RedisError::UnknownResult)
///         }
///     }
/// }
/// ```
#[proc_macro_derive(TryGetEnumVariant, attributes(error))]
pub fn try_get_enum_variant(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    macros::try_get_enum::try_get_enum_variant(input)
        .unwrap_or_else(|error| error.into_compile_error())
        .into()
}

/// Generates a `Permission` enum whose variants follow the shape
/// `{Entity}{PermissionName}{Scope}` — matching the string tokens hyperswitch
/// expects when calling `POST /user/internal/authorize_token`.
///
/// Each permission declares its own `scopes` and `entities`; both are validated
/// against the known sets. Extend the allowed sets by editing the internal
/// `EntityType` / `Scope` enums in `macros/generate_permissions.rs`.
///
/// # Example
///
/// ```ignore
/// recon_derive::generate_permissions! {
///     permissions: [
///         ReconIngestion: { scopes: [Read, Write], entities: [Profile] },
///         ReconRule:      { scopes: [Read, Write], entities: [Profile] },
///     ]
/// }
/// // expands to: pub enum Permission { ProfileReconIngestionRead, ProfileReconIngestionWrite, ... }
/// ```
#[proc_macro]
pub fn generate_permissions(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed =
        syn::parse_macro_input!(input as macros::generate_permissions::GeneratePermissionsInput);
    macros::generate_permissions::generate_permissions(parsed).into()
}
