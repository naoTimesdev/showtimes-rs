//! A custom derive macro for Meilisearch models
//!
//! Since the provided one by Meilisearch is a bit more limited for my use case.

use proc_macro::TokenStream;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, Expr, Lit, Meta, Token};

struct SearchModelAttr {
    // name = "Project"
    name: String,
    // filterable = ["id"]
    filterable: Vec<String>,
    // searchable = ["name"]
    searchable: Vec<String>,
    sortable: Vec<String>,
    displayed: Vec<String>,
    distinct: Option<String>,
}

impl Default for SearchModelAttr {
    fn default() -> Self {
        SearchModelAttr {
            name: String::new(),
            filterable: Vec::new(),
            searchable: Vec::new(),
            sortable: Vec::new(),
            displayed: vec!["*".to_string()],
            distinct: None,
        }
    }
}

fn extract_array_ident(value: Expr) -> Result<Vec<String>, syn::Error> {
    let mut result = Vec::new();
    if let Expr::Array(ref filterable) = value {
        for filter in &filterable.elems {
            if let Expr::Lit(filter) = filter {
                if let Lit::Str(filter) = &filter.lit {
                    result.push(filter.value());
                }
            }
        }
    }

    Ok(result)
}

fn get_searchmodel_attr(attrs: Vec<Attribute>) -> Result<SearchModelAttr, syn::Error> {
    let mut model_name = String::new();
    let mut model_filters = Vec::new();
    let mut model_searchable = vec!["*".to_string()];
    let mut model_sortable = Vec::new();
    let mut model_displayed = vec!["*".to_string()];
    let mut model_distinct = None;

    for attr in &attrs {
        if attr.path().is_ident("search") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for meta in nested {
                if let Meta::NameValue(nameval) = meta {
                    if nameval.path.is_ident("name") && model_name.is_empty() {
                        if let Expr::Lit(name) = nameval.value {
                            if let Lit::Str(name) = name.lit {
                                model_name = name.value();
                            }
                        }
                    } else if nameval.path.is_ident("filterable") {
                        let filterable = extract_array_ident(nameval.value)?;
                        model_filters = filterable;
                    } else if nameval.path.is_ident("searchable") {
                        let searchable = extract_array_ident(nameval.value)?;
                        if !searchable.is_empty() {
                            model_searchable = searchable;
                        }
                    } else if nameval.path.is_ident("sortable") {
                        let sortable = extract_array_ident(nameval.value)?;
                        model_sortable = sortable;
                    } else if nameval.path.is_ident("displayed") {
                        let displayed = extract_array_ident(nameval.value)?;
                        if !displayed.is_empty() {
                            model_displayed = displayed;
                        }
                    } else if nameval.path.is_ident("distinct") {
                        if let Expr::Lit(name) = nameval.value {
                            if let Lit::Str(name) = name.lit {
                                model_distinct = Some(name.value());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(SearchModelAttr {
        name: model_name,
        filterable: model_filters,
        searchable: model_searchable,
        sortable: model_sortable,
        displayed: model_displayed,
        distinct: model_distinct,
    })
}

/// The main function to expand the `SearchModel` derive macro
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
pub(crate) fn expand_searchmodel(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let (model_attr, pk_field) = match &ast.data {
        syn::Data::Struct(data) => {
            // Get the fields of the struct
            let fields = match &data.fields {
                syn::Fields::Named(fields) => fields,
                _ => {
                    return syn::Error::new(
                        ast.span(),
                        "Only structs with named fields are supported",
                    )
                    .to_compile_error()
                    .into();
                }
            };

            let pk_field = fields.named.iter().find(|&field| {
                field
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("primary_key"))
            });

            if pk_field.is_none() {
                return syn::Error::new(
                    ast.span(),
                    "The #[primary_key] field is required for the `SearchModel` derive macro",
                )
                .to_compile_error()
                .into();
            }

            // Get the search model attributes
            let search_attrs = get_searchmodel_attr(ast.attrs.clone());

            match search_attrs {
                Ok(data) => (data, pk_field.unwrap()),
                Err(err) => return err.to_compile_error().into(),
            }
        }
        _ => {
            return TokenStream::from(
                syn::Error::new_spanned(
                    name,
                    "Expected a struct for the `SearchModel` derive macro",
                )
                .to_compile_error(),
            );
        }
    };

    let mut model_attr_name = model_attr.name.clone();
    if model_attr_name.is_empty() {
        model_attr_name = name.to_string();
    }

    let model_attr_filter = model_attr.filterable.clone();
    let model_attr_search = model_attr.searchable.clone();
    let model_attr_sort = model_attr.sortable.clone();
    let model_attr_display = model_attr.displayed.clone();
    let model_attr_distinct = model_attr.distinct.clone().unwrap_or_default();

    // Convert pk_name to be able to select one the actual field in the struct
    let pk_select = pk_field
        .ident
        .as_ref()
        .ok_or_else(|| {
            syn::Error::new(pk_field.span(), "Primary key field must have an identifier")
                .to_compile_error()
        })
        .unwrap();
    let pk_field_name = pk_select.to_string();

    // Get the field type of the primary key
    let pk_field_type = pk_field.ty.clone();

    let name_str_upper = name.to_string().to_uppercase();
    let static_lock_name_str = format!("{}_INDEX_LOCK", name_str_upper);
    let static_lock_ident = syn::Ident::new(&static_lock_name_str, name.span());

    let expanded = quote::quote! {
        static #static_lock_ident: std::sync::OnceLock<meilisearch_sdk::indexes::Index> = std::sync::OnceLock::new();

        impl #name {
            /// Get the index name of the model
            pub fn index_name() -> &'static str {
                #model_attr_name
            }

            /// Get the filterable attributes of the model
            pub fn search_filterable() -> &'static [&'static str] {
                &[#(#model_attr_filter),*]
            }

            /// Get the searchable attributes of the model
            pub fn search_searchable() -> &'static [&'static str] {
                &[#(#model_attr_search),*]
            }

            /// Get the sortable attributes of the model
            pub fn search_sortable() -> &'static [&'static str] {
                &[#(#model_attr_sort),*]
            }

            /// Get the displayed attributes of the model
            pub fn search_displayed() -> &'static [&'static str] {
                &[#(#model_attr_display),*]
            }

            /// Get the distinct attribute of the model
            pub fn search_distinct() -> Option<&'static str> {
                if #model_attr_distinct.is_empty() {
                    None
                } else {
                    Some(#model_attr_distinct)
                }
            }

            /// Get the primary key of the model
            pub fn primary_key() -> &'static str {
                #pk_field_name
            }

            /// Get the primary key value of the model
            pub fn primary_key_value(&self) -> &#pk_field_type {
                &self.#pk_select
            }

            async fn set_filterable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_filterable();
                tracing::debug!("Setting filterable attributes for `{}`: {:?}", #model_attr_name, &data);
                let task = index.set_filterable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_searchable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_searchable();
                tracing::debug!("Setting searchable attributes for `{}`: {:?}", #model_attr_name, &data);
                let task = index.set_searchable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_sortable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_sortable();
                tracing::debug!("Setting sortable attributes for `{}`: {:?}", #model_attr_name, &data);
                let task = index.set_sortable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_displayed_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_displayed();
                tracing::debug!("Setting displayed attributes for `{}`: {:?}", #model_attr_name, &data);
                let task = index.set_displayed_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_distinct_attribute(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_distinct();
                let task = if let Some(data) = data {
                    tracing::debug!("Setting distinct attribute for `{}`: {:?}", #model_attr_name, &data);
                    index.set_distinct_attribute(data).await?
                } else {
                    tracing::debug!("Resetting distinct attribute for `{}`", #model_attr_name);
                    index.reset_distinct_attribute().await?
                };
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_primary_key(client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let mut index = client.index(#model_attr_name);
                tracing::debug!("Setting primary key for `{}`: {:?}", #model_attr_name, #pk_field_name);
                let task = index.set_primary_key(#pk_field_name).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            /// Update the schema of the index according to the model attributes
            pub async fn update_schema(client: &std::sync::Arc<meilisearch_sdk::client::Client>) -> Result<(), meilisearch_sdk::errors::Error> {
                tracing::debug!("Updating schema for index: {}", #model_attr_name);
                let index = client.index(#model_attr_name);

                #name::set_filterable_attributes(&index, client.deref()).await?;
                #name::set_searchable_attributes(&index, client.deref()).await?;
                #name::set_sortable_attributes(&index, client.deref()).await?;
                #name::set_displayed_attributes(&index, client.deref()).await?;
                #name::set_distinct_attribute(&index, client.deref()).await?;
                #name::set_primary_key(client.deref()).await?;

                Ok(())
            }

            /// Get the index if it exists, otherwise create it
            ///
            /// Arguments:
            /// - `client`: The MeiliSearch client
            pub async fn get_index(client: &std::sync::Arc<meilisearch_sdk::client::Client>) -> Result<meilisearch_sdk::indexes::Index, meilisearch_sdk::errors::Error> {
                tracing::debug!("Getting index: {}", #model_attr_name);
                if let Some(index_lock) = #static_lock_ident.get() {
                    return Ok(index_lock.clone());
                }

                let index = client.get_index(#model_attr_name).await;
                match index {
                    Ok(index) => {
                        if let Err(_) = #static_lock_ident.set(index.clone()) {
                            tracing::debug!("Index \"{}\" already set into OnceLock, ignoring...", #model_attr_name);
                        }
                        Ok(index)
                    },
                    Err(meilisearch_sdk::errors::Error::Meilisearch(error)) => {
                        if error.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound {
                            tracing::debug!("Index \"{}\" not found, creating...", #model_attr_name);
                            let task = client.create_index(#model_attr_name, Some(#pk_field_name)).await?;
                            tracing::debug!("Waiting for \"{}\" index creation to complete...", #model_attr_name);
                            task.wait_for_completion(client, None, None).await?;
                            tracing::debug!("Index \"{}\" created, getting the index...", #model_attr_name);
                            let index = client.get_index(#model_attr_name).await?;
                            // If not set, set it. If already set ignore!
                            if let Err(_) = #static_lock_ident.set(index.clone()) {
                                tracing::debug!("Index \"{}\" already set into OnceLock, ignoring...", #model_attr_name);
                            }
                            Ok(index)
                        } else {
                            // trickle down the error
                            Err(meilisearch_sdk::errors::Error::Meilisearch(error))
                        }
                    }
                    Err(e) => Err(e),
                }
            }

            /// Update or add this single document in the index
            ///
            /// Arguments:
            /// - `client`: The MeiliSearch client
            pub async fn update_document(&self, client: &std::sync::Arc<meilisearch_sdk::client::Client>) -> Result<(), meilisearch_sdk::errors::Error> {
                let index = #name::get_index(client).await?;
                tracing::debug!("Updating document in index: {}", #model_attr_name);
                let pvalue = self.primary_key_value().to_string();
                let task = index.add_or_update(&[self.clone()], Some(#name::primary_key())).await?;
                tracing::debug!("Waiting for document update of {:?} in \"{}\" to complete...", &pvalue, #model_attr_name);
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            /// Delete this single document in the index
            ///
            /// Arguments:
            /// - `client`: The MeiliSearch client
            pub async fn delete_document(&self, client: &std::sync::Arc<meilisearch_sdk::client::Client>) -> Result<(), meilisearch_sdk::errors::Error> {
                let index = #name::get_index(client).await?;
                tracing::debug!("Deleting document in index: {}", #model_attr_name);
                let pvalue = self.primary_key_value().to_string();
                let task = index.delete_document(&pvalue).await?;
                tracing::debug!("Waiting for document deletion of {:?} in \"{}\" to complete...", &pvalue, #model_attr_name);
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }
        }
    };

    expanded.into()
}
