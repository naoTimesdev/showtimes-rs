#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod events;
mod search;
mod shows;

/// Derive the `DbModel` trait for a struct
///
/// # Examples
/// ```
/// #[derive(ShowModelHandler)]
/// #[col_name = "ShowtimesProject"]
/// pub struct Project {
///     _id: Option<mongodb::bson::oid::ObjectId>,
///     name: String,
/// }
/// ```
#[proc_macro_derive(ShowModelHandler, attributes(col_name, handler_name))]
pub fn derive_show_model_handler(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Generate the implementation of the trait
    shows::expand_showmodel(&input)
}

/// Create a handler for a model
///
/// # Examples
/// ```
/// #[derive(ShowModelHandler)]
/// #[col_name("ShowtimesProject")]
/// pub struct Project {
///     _id: Option<mongodb::bson::oid::ObjectId>,
/// }
/// create_handler!(m::Project, ProjectHandler);
/// ```
#[proc_macro]
pub fn create_handler(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as shows::CreateHandler);

    shows::expand_handler(&input)
}

/// Derive the `SearchModel` trait for a struct
///
/// # Examples
/// ```
/// #[derive(SearchModel)]
/// #[search(name = "Project", filterable = ["id"], searchable = ["name"], sortable = ["created"])]
/// pub struct Project {
///    #[primary_key]
///    id: String,
///    name: String,
///    created: i64,
/// }
/// ```
#[proc_macro_derive(SearchModel, attributes(search, primary_key))]
pub fn derive_search_model(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Generate the implementation of the trait
    search::expand_searchmodel(&input)
}

/// Derive the `EventModel` trait for a struct
///
/// # Examples
/// ```
/// #[derive(EventModel)]
/// pub struct UserCreatedEvent {
///     id: String,
///     username: String,
/// }
/// ```
#[proc_macro_derive(EventModel, attributes(events, event_copy))]
pub fn derive_event_model(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Generate the implementation of the trait
    events::expand_eventmodel(&input)
}
