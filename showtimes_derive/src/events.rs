use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{punctuated::Punctuated, Attribute, Expr, Lit, Meta, Token};

#[derive(Default, Clone, Copy)]
struct EventModelAttr {
    unref: bool,
}

fn get_eventsmodel_attr(attrs: Vec<Attribute>) -> Result<EventModelAttr, syn::Error> {
    let mut unref = false;

    for attr in &attrs {
        if attr.path().is_ident("events") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for meta in nested {
                if let Meta::NameValue(nameval) = meta {
                    if nameval.path.is_ident("unref") {
                        // Is a boolean
                        if let Expr::Lit(lit) = nameval.value {
                            if let Lit::Bool(val) = lit.lit {
                                unref = val.value;
                            } else {
                                return Err(syn::Error::new_spanned(
                                    lit,
                                    "Expected a boolean value for `unref`",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                nameval.value,
                                "Expected a boolean value for `unref`",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(EventModelAttr { unref })
}

/// The main function to expand the `EventModel` derive macro
///
/// # Examples
/// ```
/// #[derive(EventModel)]
/// pub struct UserCreatedEvent {
///     id: String,
///     username: String,
/// }
/// ```
///
/// Will generate
///
/// ```rust
/// impl UserCreatedEvent {
///     pub fn id(&self) -> &str {
///        &self.id
///     }
///
///     pub fn username(&self) -> &str {
///        &self.username
///     }
///
///     pub fn set_id(&mut self, id: impl Into<String>) {
///        self.id = id.into();
///     }
///
///     pub fn set_username(&mut self, username: impl Into<String>) {
///        self.username = username.into();
///     }
/// }
/// ```
///
/// When the field use `Option` it will generate a getter that returns `Option<&T>`
/// If setting, it will set the value to `Some(value)` with param of the T
///
pub(crate) fn expand_eventmodel(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (fields, attrs_config) = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => match get_eventsmodel_attr(ast.attrs.clone()) {
                Ok(attrs) => (fields, attrs),
                Err(err) => return TokenStream::from(err.to_compile_error()),
            },
            _ => {
                return TokenStream::from(
                    syn::Error::new_spanned(
                        ast,
                        "Expected a struct with named fields for the `EventModel` derive macro",
                    )
                    .to_compile_error(),
                );
            }
        },
        _ => {
            return TokenStream::from(
                syn::Error::new_spanned(ast, "Expected a struct for the `EventModel` derive macro")
                    .to_compile_error(),
            );
        }
    };

    let mut getters: Vec<proc_macro2::TokenStream> = Vec::new();

    for field in fields.named.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let field_ty_name = field_ty.clone().into_token_stream().to_string();

        let field = if field_ty_name.starts_with("Option") {
            expand_option_field(field, field_name, attrs_config)
        } else {
            expand_regular_field(field, field_name, attrs_config)
        };

        getters.push(field);
    }

    let expanded = quote::quote! {
        impl #name {
            #(#getters)*
        }
    };

    expanded.into()
}

fn expand_option_field(
    field: &syn::Field,
    field_name: &syn::Ident,
    attrs_config: EventModelAttr,
) -> proc_macro2::TokenStream {
    let field_ty = &field.ty;
    let field_ty_name = field_ty.clone().into_token_stream().to_string();

    let set_field_name = format!("set_{}", field_name);
    let set_field_ident = syn::Ident::new(&set_field_name, field_name.span());

    let clear_field_name = format!("clear_{}", field_name);
    let clear_field_ident = syn::Ident::new(&clear_field_name, field_name.span());

    let (doc_get, doc_set) = make_field_comment(field_name, true);
    let doc_clear = format!("Clear the value of `{}` to [`None`]", field_name);

    // If string, we can use as_deref
    if field_ty_name.contains("String") {
        let getter = quote::quote! {
            #[doc = #doc_get]
            pub fn #field_name(&self) -> Option<&str> {
                self.#field_name.as_deref()
            }

            #[doc = #doc_set]
            pub fn #set_field_ident(&mut self, #field_name: impl Into<String>) {
                self.#field_name = Some(#field_name.into());
            }

            #[doc = #doc_clear]
            pub fn #clear_field_ident(&mut self) {
                self.#field_name = None;
            }
        };

        getter
    } else {
        // Modify the field type to be a reference
        let main_type = get_inner_type_of_option(field_ty).unwrap();
        let event_copy = has_event_copy_ident(field);

        let get_field = if event_copy {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> Option<#main_type> {
                    self.#field_name
                }
            }
        } else if let Some(inner_ty) = get_inner_type_of_vec(main_type) {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> Option<&[#inner_ty]> {
                    self.#field_name.as_deref()
                }
            }
        } else {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> Option<&#main_type> {
                    self.#field_name.as_ref()
                }
            }
        };

        let set_field = if event_copy || attrs_config.unref {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: #main_type) {
                    self.#field_name = Some(#field_name);
                }
            }
        } else if let Some(inner_ty) = get_inner_type_of_vec(main_type) {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: &[#inner_ty]) {
                    self.#field_name = Some(#field_name.to_vec());
                }
            }
        } else {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: &#main_type) {
                    self.#field_name = Some(#field_name.clone());
                }
            }
        };

        // And the set ident to be just the field type without the Option
        let getter = quote::quote! {
            #get_field

            #set_field

            #[doc = #doc_clear]
            pub fn #clear_field_ident(&mut self) {
                self.#field_name = None;
            }
        };

        getter
    }
}

fn expand_regular_field(
    field: &syn::Field,
    field_name: &syn::Ident,
    attrs_config: EventModelAttr,
) -> proc_macro2::TokenStream {
    let field_ty = &field.ty;
    let field_ty_name = field_ty.clone().into_token_stream().to_string();

    let set_field_name = format!("set_{}", field_name);
    let set_field_ident = syn::Ident::new(&set_field_name, field_name.span());

    let (doc_get, doc_set) = make_field_comment(field_name, false);

    // If string, we can use as_deref
    if field_ty_name.contains("String") {
        let getter = quote::quote! {
            #[doc = #doc_get]
            pub fn #field_name(&self) -> &str {
                &self.#field_name
            }

            #[doc = #doc_set]
            pub fn #set_field_ident(&mut self, #field_name: impl Into<String>) {
                self.#field_name = #field_name.into();
            }
        };

        getter
    } else {
        let event_copy = has_event_copy_ident(field);

        let get_field = if event_copy {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> #field_ty {
                    self.#field_name
                }
            }
        } else if let Some(inner_ty) = get_inner_type_of_vec(field_ty) {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> &[#inner_ty] {
                    &self.#field_name
                }
            }
        } else {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> &#field_ty {
                    &self.#field_name
                }
            }
        };

        let set_field = if event_copy || attrs_config.unref {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: #field_ty) {
                    self.#field_name = #field_name;
                }
            }
        } else if let Some(inner_ty) = get_inner_type_of_vec(field_ty) {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: &[#inner_ty]) {
                    self.#field_name = #field_name.to_vec();
                }
            }
        } else {
            quote::quote! {
                #[doc = #doc_set]
                pub fn #set_field_ident(&mut self, #field_name: &#field_ty) {
                    self.#field_name = #field_name.clone();
                }
            }
        };

        let getter = quote::quote! {
            #get_field

            #set_field
        };

        getter
    }
}

fn get_inner_type_of_x<'a>(ty: &'a syn::Type, x: &'a str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        // Check if it's a path type, and the first segment of the path is "x"
        if let Some(segment) = type_path.path.segments.first() {
            if segment.ident == x {
                // Check if the segment has generic arguments (i.e., x<T>)
                if let syn::PathArguments::AngleBracketed(angle_bracketed) = &segment.arguments {
                    // Get the first generic argument (T in x<T>)
                    if let Some(syn::GenericArgument::Type(inner_type)) =
                        angle_bracketed.args.first()
                    {
                        return Some(inner_type);
                    }
                }
            }
        }
    }
    None
}

fn get_inner_type_of_option(ty: &syn::Type) -> Option<&syn::Type> {
    get_inner_type_of_x(ty, "Option")
}

fn get_inner_type_of_vec(ty: &syn::Type) -> Option<&syn::Type> {
    get_inner_type_of_x(ty, "Vec")
}

fn has_event_copy_ident(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("event_copy"))
}

/// Generate field comment
///
/// If `option_mode` use the "if it exists" comment
fn make_field_comment(field: &syn::Ident, option_mode: bool) -> (String, String) {
    let if_it_exists = if option_mode { " if it exists" } else { "" };
    let doc_get = format!("Get the value of `{}`{}", field, if_it_exists);
    let doc_set = format!("Set the value of `{}` to the given value", field);

    (doc_get, doc_set)
}
