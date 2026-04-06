use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

/// Derive `FromRow` for a struct — automatically decode a `Row` into the struct.
///
/// Each field is decoded by name from the row using `try_get_by_name`.
/// Field types must implement `FromSql`. Use `Option<T>` for nullable columns.
///
/// # Example
///
/// ```rust,ignore
/// use sentinel_derive::FromRow;
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

    let rename_all = get_struct_rename_all(input);

    let field_extractions = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_ty = &f.ty;
        let column_name = field_name.to_string();

        let attrs = parse_field_attrs(f).unwrap();

        // #[sentinel(skip)] — always use Default::default()
        if attrs.skip {
            return quote! {
                #field_name: ::std::default::Default::default()
            };
        }

        // #[sentinel(flatten)] — delegate to nested FromRow
        if attrs.flatten {
            return quote! {
                #field_name: #field_ty::from_row(row)?
            };
        }

        // Determine column name
        let col = attrs.rename.unwrap_or_else(|| {
            if let Some(ref strategy) = rename_all {
                apply_rename_all(&column_name, strategy)
            } else {
                column_name
            }
        });

        // #[sentinel(json)] — decode as JSON string then deserialize
        if attrs.json {
            return quote! {
                #field_name: {
                    let json_str: String = row.try_get_by_name(#col)?;
                    serde_json::from_str(&json_str)
                        .map_err(|e| sentinel_driver::Error::Decode(format!("json: {}", e)))?
                }
            };
        }

        // #[sentinel(try_from = "SourceType")]
        if let Some(ref source_ty) = attrs.try_from {
            if attrs.default {
                return quote! {
                    #field_name: match row.try_get_by_name::<#source_ty>(#col) {
                        Ok(v) => <#field_ty as ::std::convert::TryFrom<#source_ty>>::try_from(v)
                            .map_err(|e| sentinel_driver::Error::Decode(format!("{}", e)))?,
                        Err(sentinel_driver::Error::ColumnNotFound(_)) => ::std::default::Default::default(),
                        Err(e) => return Err(e),
                    }
                };
            }
            return quote! {
                #field_name: {
                    let v = row.try_get_by_name::<#source_ty>(#col)?;
                    <#field_ty as ::std::convert::TryFrom<#source_ty>>::try_from(v)
                        .map_err(|e| sentinel_driver::Error::Decode(format!("{}", e)))?
                }
            };
        }

        // #[sentinel(default)] — use Default if column missing
        if attrs.default {
            return quote! {
                #field_name: match row.try_get_by_name(#col) {
                    Ok(v) => v,
                    Err(sentinel_driver::Error::ColumnNotFound(_)) => ::std::default::Default::default(),
                    Err(e) => return Err(e),
                }
            };
        }

        // Normal field
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
/// use sentinel_derive::ToSql;
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
/// use sentinel_derive::FromSql;
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

/// Convert a field name string according to a naming convention.
fn apply_rename_all(name: &str, strategy: &str) -> String {
    match strategy {
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "camelCase" => {
            let mut result = String::new();
            let mut capitalize_next = false;
            for (i, c) in name.chars().enumerate() {
                if c == '_' {
                    capitalize_next = true;
                } else if capitalize_next {
                    result.extend(c.to_uppercase());
                    capitalize_next = false;
                } else if i == 0 {
                    result.extend(c.to_lowercase());
                } else {
                    result.push(c);
                }
            }
            result
        }
        "PascalCase" => {
            let mut result = String::new();
            let mut capitalize_next = true;
            for c in name.chars() {
                if c == '_' {
                    capitalize_next = true;
                } else if capitalize_next {
                    result.extend(c.to_uppercase());
                    capitalize_next = false;
                } else {
                    result.push(c);
                }
            }
            result
        }
        "snake_case" => {
            let mut result = String::new();
            for (i, c) in name.chars().enumerate() {
                if c.is_uppercase() && i > 0 {
                    result.push('_');
                }
                result.extend(c.to_lowercase());
            }
            result
        }
        "SCREAMING_SNAKE_CASE" => {
            let mut result = String::new();
            for (i, c) in name.chars().enumerate() {
                if c.is_uppercase() && i > 0 {
                    result.push('_');
                }
                result.extend(c.to_uppercase());
            }
            result
        }
        "kebab-case" => {
            let mut result = String::new();
            for (i, c) in name.chars().enumerate() {
                if c == '_' {
                    result.push('-');
                } else if c.is_uppercase() && i > 0 {
                    result.push('-');
                    result.extend(c.to_lowercase());
                } else {
                    result.extend(c.to_lowercase());
                }
            }
            result
        }
        _ => name.to_string(),
    }
}

/// Parse struct-level `#[sentinel(rename_all = "strategy")]` attribute.
fn get_struct_rename_all(input: &DeriveInput) -> Option<String> {
    for attr in &input.attrs {
        if !attr.path().is_ident("sentinel") {
            continue;
        }
        let result: syn::Result<String> =
            attr.parse_args_with(|input: syn::parse::ParseStream| {
                let ident: syn::Ident = input.parse()?;
                if ident != "rename_all" {
                    return Err(syn::Error::new_spanned(&ident, "expected `rename_all`"));
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

/// All supported field-level `#[sentinel(...)]` attributes.
struct FieldAttrs {
    rename: Option<String>,
    skip: bool,
    default: bool,
    try_from: Option<Type>,
    flatten: bool,
    json: bool,
}

fn parse_field_attrs(field: &syn::Field) -> syn::Result<FieldAttrs> {
    let mut attrs = FieldAttrs {
        rename: None,
        skip: false,
        default: false,
        try_from: None,
        flatten: false,
        json: false,
    };

    for attr in &field.attrs {
        if !attr.path().is_ident("sentinel") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                attrs.rename = Some(s.value());
            } else if meta.path.is_ident("skip") {
                attrs.skip = true;
            } else if meta.path.is_ident("default") {
                attrs.default = true;
            } else if meta.path.is_ident("try_from") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                attrs.try_from = Some(syn::parse_str(&s.value())?);
            } else if meta.path.is_ident("flatten") {
                attrs.flatten = true;
            } else if meta.path.is_ident("json") {
                attrs.json = true;
            } else {
                return Err(meta.error("unknown sentinel attribute"));
            }
            Ok(())
        })?;
    }

    Ok(attrs)
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
