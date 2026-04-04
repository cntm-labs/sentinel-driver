use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Type};

/// Derive `FromRow` for a struct — automatically decode a `Row` into the struct.
///
/// Each field is decoded by name from the row using `try_get_by_name`.
/// Field types must implement `FromSql`. Use `Option<T>` for nullable columns.
///
/// # Example
///
/// ```rust,ignore
/// use sentinel_driver_derive::FromRow;
///
/// #[derive(FromRow)]
/// struct User {
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// let row = conn.query_one("SELECT id, name, email FROM users WHERE id = $1", &[&1]).await?;
/// let user = User::from_row(&row)?;
/// ```
#[proc_macro_derive(FromRow, attributes(sentinel))]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_from_row(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_from_row(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "FromRow can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "FromRow can only be derived for structs",
            ))
        }
    };

    let field_extractions = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let column_name = field_name.to_string();

        // Check for #[sentinel(rename = "...")] attribute
        let col = get_rename_attr(f).unwrap_or(column_name);

        quote! {
            #field_name: row.try_get_by_name(#col)?
        }
    });

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Decode a `Row` into this struct.
            pub fn from_row(row: &sentinel_driver::Row) -> sentinel_driver::Result<Self> {
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }
    })
}

/// Derive `ToSql` for a newtype wrapper.
///
/// The struct must have exactly one field that implements `ToSql`.
///
/// # Example
///
/// ```rust,ignore
/// use sentinel_driver_derive::ToSql;
///
/// #[derive(ToSql)]
/// struct UserId(i32);
/// ```
#[proc_macro_derive(ToSql)]
pub fn derive_to_sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_to_sql(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_to_sql(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Validate this is a newtype struct
    get_single_field(input, "ToSql")?;

    Ok(quote! {
        impl #impl_generics sentinel_driver::ToSql for #name #ty_generics #where_clause {
            fn oid(&self) -> sentinel_driver::Oid {
                self.0.oid()
            }

            fn to_sql(&self, buf: &mut bytes::BytesMut) -> sentinel_driver::Result<()> {
                self.0.to_sql(buf)
            }
        }
    })
}

/// Derive `FromSql` for a newtype wrapper.
///
/// The struct must have exactly one field that implements `FromSql`.
///
/// # Example
///
/// ```rust,ignore
/// use sentinel_driver_derive::FromSql;
///
/// #[derive(FromSql)]
/// struct UserId(i32);
/// ```
#[proc_macro_derive(FromSql)]
pub fn derive_from_sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_from_sql(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_from_sql(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let inner_ty = get_single_field(input, "FromSql")?;

    Ok(quote! {
        impl #impl_generics sentinel_driver::FromSql for #name #ty_generics #where_clause {
            fn oid() -> sentinel_driver::Oid {
                <#inner_ty as sentinel_driver::FromSql>::oid()
            }

            fn from_sql(buf: &[u8]) -> sentinel_driver::Result<Self> {
                <#inner_ty as sentinel_driver::FromSql>::from_sql(buf).map(Self)
            }
        }
    })
}

// ── Helpers ──────────────────────────────────────────

/// Extract the inner type from a newtype (struct with exactly one unnamed field).
fn get_single_field(input: &DeriveInput, derive_name: &str) -> syn::Result<Type> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                Ok(fields.unnamed.first().unwrap().ty.clone())
            }
            _ => Err(syn::Error::new_spanned(
                input,
                format!(
                    "{derive_name} can only be derived for newtype structs (e.g., struct Foo(i32))"
                ),
            )),
        },
        _ => Err(syn::Error::new_spanned(
            input,
            format!("{derive_name} can only be derived for structs"),
        )),
    }
}

/// Check for `#[sentinel(rename = "column_name")]` attribute on a field.
fn get_rename_attr(field: &syn::Field) -> Option<String> {
    for attr in &field.attrs {
        if !attr.path().is_ident("sentinel") {
            continue;
        }

        let result: syn::Result<String> = attr.parse_args_with(|input: syn::parse::ParseStream| {
            let ident: syn::Ident = input.parse()?;
            if ident != "rename" {
                return Err(syn::Error::new_spanned(&ident, "expected `rename`"));
            }
            let _: syn::Token![=] = input.parse()?;
            let lit: syn::LitStr = input.parse()?;
            Ok(lit.value())
        });

        if let Ok(name) = result {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    // Proc macro tests need to be integration tests or use trybuild.
    // Unit tests here verify helper logic only.

    #[test]
    fn test_crate_compiles() {
        // If this compiles, the proc macro crate is valid
        assert!(true);
    }
}
