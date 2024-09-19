use proc_macro::TokenStream;
use quote::ToTokens;

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

    let fields = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields,
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
            expand_option_field(field, field_name)
        } else {
            expand_regular_field(field, field_name)
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

fn expand_option_field(field: &syn::Field, field_name: &syn::Ident) -> proc_macro2::TokenStream {
    let field_ty = &field.ty;
    let field_ty_name = field_ty.clone().into_token_stream().to_string();

    let set_field_name = format!("set_{}", field_name);
    let set_field_ident = syn::Ident::new(&set_field_name, field_name.span());
    let (doc_get, doc_set) = make_field_comment(field_name, true);

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
        };

        getter
    } else {
        // Modify the field type to be a reference
        let main_type = get_inner_type_of_option(field_ty).unwrap();

        let get_field = if has_event_copy_ident(field) {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> Option<#main_type> {
                    self.#field_name
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

        // And the set ident to be just the field type without the Option
        let getter = quote::quote! {
            #get_field

            #[doc = #doc_set]
            pub fn #set_field_ident(&mut self, #field_name: #main_type) {
                self.#field_name = Some(#field_name);
            }
        };

        getter
    }
}

fn expand_regular_field(field: &syn::Field, field_name: &syn::Ident) -> proc_macro2::TokenStream {
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
        let get_field = if has_event_copy_ident(field) {
            quote::quote! {
                #[doc = #doc_get]
                pub fn #field_name(&self) -> #field_ty {
                    self.#field_name
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

        let getter = quote::quote! {
            #get_field

            #[doc = #doc_set]
            pub fn #set_field_ident(&mut self, #field_name: #field_ty) {
                self.#field_name = #field_name;
            }
        };

        getter
    }
}

fn get_inner_type_of_option(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        // Check if it's a path type, and the first segment of the path is "Option"
        if let Some(segment) = type_path.path.segments.first() {
            if segment.ident == "Option" {
                // Check if the segment has generic arguments (i.e., Option<T>)
                if let syn::PathArguments::AngleBracketed(angle_bracketed) = &segment.arguments {
                    // Get the first generic argument (T in Option<T>)
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
    let doc_get = format!("Get the value of {}{}", field, if_it_exists);
    let doc_set = format!("Set the value of {} to the given value", field);

    (doc_get, doc_set)
}
