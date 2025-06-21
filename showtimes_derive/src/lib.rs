#![warn(missing_docs, clippy::empty_docs, rustdoc::broken_intra_doc_links)]
#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod enums;
mod events;
mod search;

/// Derive the `SearchModel` trait for a struct
///
/// # Examples
/// ```
/// # use showtimes_derive::SearchModel;
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
/// # use showtimes_derive::EventModel;
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

/// Derive the `EnumName` trait for an enum
///
/// # Examples
/// ```
/// # use showtimes_derive::EnumName;
/// #[derive(EnumName)]
/// pub enum ProjectStatus {
///     Active,
///     Completed,
/// }
/// ```
#[proc_macro_derive(EnumName, attributes(enum_name))]
pub fn derive_enum_name(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Generate the implementation of the trait
    enums::expand_enumname(&input)
}

/// Derive the `SerdeAutomata` trait for an enum
///
/// This will help create a [`serde::Serialize`] and [`serde::Deserialize`] implementation
/// for an enum, you can also customize it with the `serde_automata` attribute
///
/// # Examples
/// ```
/// # use showtimes_derive::SerdeAutomata;
/// #[derive(SerdeAutomata)]
/// pub enum ProjectStatus {
///     Active,
///     Completed,
/// }
/// ```
#[proc_macro_derive(SerdeAutomata, attributes(serde_automata))]
pub fn derive_serde_automata(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    // Generate the implementation of the trait
    enums::serde_automata_expand(&input)
}
