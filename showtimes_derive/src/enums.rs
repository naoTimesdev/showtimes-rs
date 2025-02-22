use ahash::{HashMap, HashMapExt};
use convert_case::Casing;
use proc_macro::TokenStream;
use syn::{Attribute, Expr, Lit, LitStr, Meta, Token, punctuated::Punctuated, spanned::Spanned};

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
                        match nameval.value {
                            Expr::Lit(lit) => match lit.lit {
                                Lit::Str(val) => {
                                    rename = Some(val.value());
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        lit,
                                        "Expected a string value for `rename`",
                                    ));
                                }
                            },
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nameval.value,
                                    "Expected a string value for `rename`",
                                ));
                            }
                        }
                    } else if nameval.path.is_ident("rename_all") {
                        // Is a string
                        match nameval.value {
                            Expr::Lit(lit) => match lit.lit {
                                Lit::Str(val) => {
                                    rename_all = map_convert_case(&val.value(), &val)?;
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        lit,
                                        "Expected a string value for `rename_all`",
                                    ));
                                }
                            },
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nameval.value,
                                    "Expected a string value for `rename_all`",
                                ));
                            }
                        }
                    } else if nameval.path.is_ident("strict") {
                        // Is a boolean
                        match nameval.value {
                            Expr::Lit(lit) => match lit.lit {
                                Lit::Bool(val) => {
                                    strict = val.value;
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        lit,
                                        "Expected a boolean value for `strict`",
                                    ));
                                }
                            },
                            _ => {
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
                .into();
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

#[derive(Debug, Clone)]
struct SerdeAutomataAttr {
    /// Rename field to
    rename: Vec<String>,
    ser_rename: Vec<String>,
    deser_rename: Vec<String>,
    /// Globally convert field to
    rename_all: convert_case::Case,
    /// Deserialize field rename
    deserialize_rename_all: Option<convert_case::Case>,
    /// For serializing
    serialize_rename_all: Option<convert_case::Case>,
    /// Strict mode, default true
    ///
    /// Will check if all variants have unique values
    strict: bool,
    skip: bool,
    case_sensitive: bool,
}

impl Default for SerdeAutomataAttr {
    fn default() -> Self {
        SerdeAutomataAttr {
            rename: vec![],
            ser_rename: vec![],
            deser_rename: vec![],
            rename_all: convert_case::Case::Pascal,
            deserialize_rename_all: None,
            serialize_rename_all: None,
            strict: true,
            skip: false,
            case_sensitive: false,
        }
    }
}

fn split_lit_str(lit: &LitStr) -> Vec<String> {
    lit.value()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect()
}

fn get_serde_automata_attr(attrs: &[Attribute]) -> Result<SerdeAutomataAttr, syn::Error> {
    let mut rename = vec![];
    let mut ser_rename = vec![];
    let mut deser_rename = vec![];
    let mut rename_all = convert_case::Case::Pascal;
    let mut deserialize_rename_all = None;
    let mut serialize_rename_all = None;
    let mut strict = true;
    let mut case_sensitive = false;
    let mut skip = false;

    for attr in attrs {
        if attr.path().is_ident("serde_automata") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in nested {
                match meta {
                    Meta::NameValue(nameval) => {
                        if nameval.path.is_ident("rename") {
                            // Is a string
                            match nameval.value {
                                Expr::Lit(lit) => {
                                    match lit.lit {
                                        Lit::Str(val) => {
                                            // Split by comma, then trim whitespace
                                            rename = split_lit_str(&val);
                                        }
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                lit,
                                                "Expected a string value for `rename`",
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a string value for `rename`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("ser_rename") {
                            // Is a string
                            match nameval.value {
                                Expr::Lit(lit) => {
                                    match lit.lit {
                                        Lit::Str(val) => {
                                            // Split by comma, then trim whitespace
                                            ser_rename = split_lit_str(&val);
                                        }
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                lit,
                                                "Expected a string value for `ser_rename`",
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a string value for `ser_rename`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("deser_rename") {
                            // Is a string
                            match nameval.value {
                                Expr::Lit(lit) => {
                                    match lit.lit {
                                        Lit::Str(val) => {
                                            // Split by comma, then trim whitespace
                                            deser_rename = split_lit_str(&val);
                                        }
                                        _ => {
                                            return Err(syn::Error::new_spanned(
                                                lit,
                                                "Expected a string value for `deser_rename`",
                                            ));
                                        }
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a string value for `deser_rename`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("rename_all") {
                            // Is a string
                            match nameval.value {
                                Expr::Lit(lit) => match lit.lit {
                                    Lit::Str(val) => {
                                        rename_all = map_convert_case(&val.value(), &val)?;
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            lit,
                                            "Expected a string value for `rename_all`",
                                        ));
                                    }
                                },
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a string value for `rename_all`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("strict") {
                            // Is a boolean
                            match nameval.value {
                                Expr::Lit(lit) => match lit.lit {
                                    Lit::Bool(val) => {
                                        strict = val.value;
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            lit,
                                            "Expected a boolean value for `strict`",
                                        ));
                                    }
                                },
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a boolean value for `strict`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("case_sensitive") {
                            // Is a boolean
                            match nameval.value {
                                Expr::Lit(lit) => match lit.lit {
                                    Lit::Bool(val) => {
                                        case_sensitive = val.value;
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            lit,
                                            "Expected a boolean value for `case_sensitive`",
                                        ));
                                    }
                                },
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        nameval.value,
                                        "Expected a boolean value for `case_sensitive`",
                                    ));
                                }
                            }
                        } else if nameval.path.is_ident("deserialize_rename_all") {
                            // Is a string
                            if let Expr::Lit(lit) = nameval.value {
                                match lit.lit {
                                    Lit::Str(val) => {
                                        deserialize_rename_all =
                                            Some(map_convert_case(&val.value(), &val)?);
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            lit,
                                            "Expected a string value for `deserialize_rename_all`",
                                        ));
                                    }
                                }
                            }
                        } else if nameval.path.is_ident("serialize_rename_all") {
                            // Is a string
                            if let Expr::Lit(lit) = nameval.value {
                                match lit.lit {
                                    Lit::Str(val) => {
                                        serialize_rename_all =
                                            Some(map_convert_case(&val.value(), &val)?);
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            lit,
                                            "Expected a string value for `serialize_rename_all`",
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        if let Meta::Path(pathval) = meta {
                            if pathval.is_ident("skip") {
                                skip = true;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(SerdeAutomataAttr {
        rename,
        ser_rename,
        deser_rename,
        rename_all,
        deserialize_rename_all,
        serialize_rename_all,
        strict,
        case_sensitive,
        skip,
    })
}

pub(crate) fn serde_automata_expand(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;

    // check if ast is an enum
    let variants = match &input.data {
        syn::Data::Enum(v) => &v.variants,
        _ => {
            return syn::Error::new(
                input.span(),
                "`SerdeAutomata` can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let root_attrs = match get_serde_automata_attr(&input.attrs) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut registered_ser: HashMap<String, &syn::Ident> = HashMap::new();
    let mut registered_deser: HashMap<String, &syn::Ident> = HashMap::new();
    let mut arms_serialize = Vec::new();
    let mut arms_deserialize = Vec::new();
    for variant in variants {
        let vardent = &variant.ident;
        let var_name = vardent.to_string();
        let variant_attrs = match get_serde_automata_attr(&variant.attrs) {
            Ok(attrs) => attrs,
            Err(err) => return err.to_compile_error().into(),
        };

        let serialize_str =
            if variant_attrs.rename.is_empty() && variant_attrs.ser_rename.is_empty() {
                if let Some(ser_case) = root_attrs.serialize_rename_all {
                    vec![var_name.to_case(ser_case)]
                } else {
                    vec![var_name.to_case(root_attrs.rename_all)]
                }
            } else if variant_attrs.ser_rename.is_empty() {
                variant_attrs.rename.clone()
            } else {
                variant_attrs.ser_rename
            };

        for ser_arm in serialize_str.iter() {
            if let Some(dupe_ident) = registered_ser.get(ser_arm) {
                if root_attrs.strict {
                    let dupe_name = dupe_ident.to_string();
                    return syn::Error::new_spanned(
                        vardent,
                        format!(
                            "Duplicate name value for serialization: `{}`, conflict with `{}`",
                            &ser_arm, &dupe_name
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
            }

            registered_ser.insert(ser_arm.clone(), vardent);
        }

        // Select first arm for serialize
        if let Some(ser_arm) = serialize_str.first() {
            arms_serialize.push(quote::quote! {
                #name::#vardent => #ser_arm,
            })
        } else {
            return syn::Error::new_spanned(
                vardent,
                format!("Unknown variant for serialization: `{}`", &vardent),
            )
            .to_compile_error()
            .into();
        }

        if variant_attrs.skip {
            // skip deser, since ser is required
            continue;
        }

        let deserialize_str =
            if variant_attrs.rename.is_empty() && variant_attrs.deser_rename.is_empty() {
                if let Some(deser_case) = root_attrs.deserialize_rename_all {
                    vec![var_name.to_case(deser_case)]
                } else {
                    vec![var_name.to_case(root_attrs.rename_all)]
                }
            } else if variant_attrs.deser_rename.is_empty() {
                variant_attrs.rename.clone()
            } else {
                variant_attrs.deser_rename
            };

        for deser_arm in deserialize_str.iter() {
            if let Some(dupe_ident) = registered_deser.get(deser_arm) {
                if root_attrs.strict {
                    let dupe_name = dupe_ident.to_string();
                    return syn::Error::new_spanned(
                        vardent,
                        format!(
                            "Duplicate name value for deserialization: `{}`, conflict with `{}`",
                            &deser_arm, &dupe_name
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
            }

            registered_deser.insert(deser_arm.clone(), vardent);
        }

        // Make each serialize to "string_a" | "string_b" => Ok(#name::#vardent)
        arms_deserialize.push(quote::quote! {
            #(#deserialize_str)|* => Ok(#name::#vardent),
        });
    }

    // Merge #name + FromStrError
    // Deserialize arms
    let deser_arm_root = if root_attrs.case_sensitive {
        quote::quote! {
            let value = String::deserialize(deserializer)?;
        }
    } else {
        quote::quote! {
            let original = String::deserialize(deserializer)?;
            let value = original.to_lowercase();
        }
    };

    let tokens = quote::quote! {
        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer
            {
                let matched = match self {
                    #(#arms_serialize)*
                };
                serializer.serialize_str(matched)
            }
        }

        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #deser_arm_root
                match value.as_str() {
                    #(#arms_deserialize)*
                    _ => Err(serde::de::Error::custom(format!("Unknown variant for deserialization: `{}`", value)))
                }
            }
        }
    };

    tokens.into()
}
