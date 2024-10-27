use std::collections::HashMap;

use convert_case::Casing;
use proc_macro::TokenStream;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Expr, Lit, LitStr, Meta, Token};

#[derive(Debug, Clone)]
struct EnumNameAttr {
    /// Rename field to
    rename: Option<String>,
    /// Globally convert field to
    rename_all: convert_case::Case,
    /// Strict mode, default true
    ///
    /// Will check if all variants have unique values
    strict: bool,
}

impl Default for EnumNameAttr {
    fn default() -> Self {
        EnumNameAttr {
            rename: None,
            rename_all: convert_case::Case::Pascal,
            strict: true,
        }
    }
}

fn map_convert_case(case: &str, expr: &LitStr) -> Result<convert_case::Case, syn::Error> {
    match case {
        "UPPERCASE" => Ok(convert_case::Case::Upper),
        "lowercase" => Ok(convert_case::Case::Lower),
        "camelCase" => Ok(convert_case::Case::Camel),
        "PascalCase" => Ok(convert_case::Case::Pascal),
        "snake_case" => Ok(convert_case::Case::Snake),
        "SCREAMING_SNAKE_CASE" => Ok(convert_case::Case::ScreamingSnake),
        "kebab-case" => Ok(convert_case::Case::Kebab),
        "flatcase" => Ok(convert_case::Case::Flat),
        "UPPERFLATCASE" => Ok(convert_case::Case::UpperFlat),
        _ => Err(syn::Error::new_spanned(
            expr,
            "Expected one of: UPPERCASE, lowercase, camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, flatcase, UPPERFLATCASE",
        )),
    }
}

fn get_enumname_attr(attrs: &[Attribute]) -> Result<EnumNameAttr, syn::Error> {
    let mut rename = None;
    let mut rename_all = convert_case::Case::Pascal;
    let mut strict = true;

    for attr in attrs {
        if attr.path().is_ident("enum_name") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in nested {
                if let Meta::NameValue(nameval) = meta {
                    if nameval.path.is_ident("rename") {
                        // Is a string
                        if let Expr::Lit(lit) = nameval.value {
                            if let Lit::Str(val) = lit.lit {
                                rename = Some(val.value());
                            } else {
                                return Err(syn::Error::new_spanned(
                                    lit,
                                    "Expected a string value for `rename`",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                nameval.value,
                                "Expected a string value for `rename`",
                            ));
                        }
                    } else if nameval.path.is_ident("rename_all") {
                        // Is a string
                        if let Expr::Lit(lit) = nameval.value {
                            if let Lit::Str(val) = lit.lit {
                                rename_all = map_convert_case(&val.value(), &val)?;
                            } else {
                                return Err(syn::Error::new_spanned(
                                    lit,
                                    "Expected a string value for `rename_all`",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                nameval.value,
                                "Expected a string value for `rename_all`",
                            ));
                        }
                    } else if nameval.path.is_ident("strict") {
                        // Is a boolean
                        if let Expr::Lit(lit) = nameval.value {
                            if let Lit::Bool(val) = lit.lit {
                                strict = val.value;
                            } else {
                                return Err(syn::Error::new_spanned(
                                    lit,
                                    "Expected a boolean value for `strict`",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                nameval.value,
                                "Expected a boolean value for `strict`",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(EnumNameAttr {
        rename,
        rename_all,
        strict,
    })
}

pub(crate) fn expand_enumname(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;

    // check if ast is an enum
    let variants = match &input.data {
        syn::Data::Enum(v) => &v.variants,
        _ => {
            return syn::Error::new(input.span(), "`EnumName` can only be derived for enums")
                .to_compile_error()
                .into()
        }
    };

    let root_attrs = match get_enumname_attr(&input.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut registered_names: HashMap<String, &syn::Ident> = HashMap::new();
    let mut arms = Vec::new();
    for variant in variants {
        let vardent = &variant.ident;
        let var_name = vardent.to_string();
        let variant_attrs = match get_enumname_attr(&variant.attrs) {
            Ok(attrs) => attrs,
            Err(err) => return err.to_compile_error().into(),
        };

        let arm_str = match variant_attrs.rename {
            Some(name) => name,
            None => var_name.to_case(root_attrs.rename_all),
        };

        if let Some(dupe_ident) = registered_names.get(&arm_str) {
            if root_attrs.strict {
                let dupe_name = dupe_ident.to_string();
                return syn::Error::new_spanned(
                    vardent,
                    format!(
                        "Duplicate name value: `{}`, conflict with `{}`",
                        &arm_str, &dupe_name
                    ),
                )
                .to_compile_error()
                .into();
            }
        }
        registered_names.insert(arm_str.clone(), vardent);

        arms.push(quote::quote! {
            #name::#vardent => #arm_str,
        });
    }

    let tokens = quote::quote! {
        impl #name {
            /// Returns the name representation of the enum
            pub fn to_name(&self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    tokens.into()
}
