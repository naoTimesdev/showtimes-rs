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

fn extract_array_ident(
    value: Expr,
    name: &str,
    fields: &Vec<String>,
) -> Result<Vec<String>, syn::Error> {
    let mut result = Vec::new();
    if let Expr::Array(ref filterable) = value {
        for filter in &filterable.elems {
            if let Expr::Lit(filter) = filter {
                if let Lit::Str(filter) = &filter.lit {
                    if !fields.contains(&filter.value()) {
                        return Err(syn::Error::new(
                            filter.span(),
                            format!("Field {} is not {}", filter.value(), name),
                        ));
                    }
                    result.push(filter.value());
                }
            }
        }
    }

    Ok(result)
}

fn get_searchmodel_attr(
    attrs: Vec<Attribute>,
    fields: &Vec<String>,
) -> Result<SearchModelAttr, syn::Error> {
    let mut model_name = String::new();
    let mut model_filters = Vec::new();
    let mut model_searchable = Vec::new();
    let mut model_sortable = Vec::new();
    let mut model_displayed = vec!["*".to_string()];
    let mut model_distinct = None;

    for attr in &attrs {
        if attr.path().is_ident("search") {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

            for meta in nested {
                match meta {
                    Meta::NameValue(nameval) => {
                        if nameval.path.is_ident("name") && model_name.is_empty() {
                            if let Expr::Lit(name) = nameval.value {
                                if let Lit::Str(name) = name.lit {
                                    model_name = name.value();
                                }
                            }
                        } else if nameval.path.is_ident("filterable") {
                            let filterable =
                                extract_array_ident(nameval.value, "filterable", fields)?;
                            model_filters = filterable;
                        } else if nameval.path.is_ident("searchable") {
                            let searchable =
                                extract_array_ident(nameval.value, "searchable", fields)?;
                            if searchable.is_empty() {
                                // Default to all fields
                                model_searchable = fields.clone();
                            } else {
                                model_searchable = searchable;
                            }
                        } else if nameval.path.is_ident("sortable") {
                            let sortable = extract_array_ident(nameval.value, "sortable", fields)?;
                            model_sortable = sortable;
                        } else if nameval.path.is_ident("displayed") {
                            let displayed =
                                extract_array_ident(nameval.value, "displayed", fields)?;
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

                    _ => {}
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

    let (model_attr, pk_name) = match &ast.data {
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

            // Get all fields names
            let field_names = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().unwrap().to_string())
                .collect::<Vec<String>>();

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

            // Get the field name of the primary key
            let pk_field_name = pk_field.unwrap().ident.as_ref().unwrap().to_string();

            // Get the search model attributes
            let search_attrs = get_searchmodel_attr(ast.attrs.clone(), &field_names);

            match search_attrs {
                Ok(data) => (data, pk_field_name),
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

    let expanded = quote::quote! {
        impl #name {
            /// Get the index name of the model
            pub fn index_name() -> &'static str {
                #model_attr_name
            }

            /// Get the filterable attributes of the model
            pub fn search_filterable() -> Vec<&'static str> {
                vec![#(#model_attr_filter),*]
            }

            /// Get the searchable attributes of the model
            pub fn search_searchable() -> Vec<&'static str> {
                vec![#(#model_attr_search),*]
            }

            /// Get the sortable attributes of the model
            pub fn search_sortable() -> Vec<&'static str> {
                vec![#(#model_attr_sort),*]
            }

            /// Get the displayed attributes of the model
            pub fn search_displayed() -> Vec<&'static str> {
                vec![#(#model_attr_display),*]
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
                #pk_name
            }

            async fn set_filterable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_filterable();
                let task = index.set_filterable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_searchable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_searchable();
                let task = index.set_searchable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_sortable_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_sortable();
                let task = index.set_sortable_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_displayed_attributes(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_displayed();
                let task = index.set_displayed_attributes(data).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_distinct_attribute(index: &meilisearch_sdk::indexes::Index, client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let data = #name::search_distinct();
                let task = if let Some(data) = data {
                    index.set_distinct_attribute(data).await?
                } else {
                    index.reset_distinct_attribute().await?
                };
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            async fn set_primary_key(client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let mut index = client.index(#model_attr_name);
                let task = index.set_primary_key(#pk_name).await?;
                task.wait_for_completion(client, None, None).await?;
                Ok(())
            }

            /// Update the schema of the index according to the model attributes
            pub async fn update_schema(client: &meilisearch_sdk::client::Client) -> Result<(), meilisearch_sdk::errors::Error> {
                let index = client.index(#model_attr_name);

                #name::set_filterable_attributes(&index, client).await?;
                #name::set_searchable_attributes(&index, client).await?;
                #name::set_sortable_attributes(&index, client).await?;
                #name::set_displayed_attributes(&index, client).await?;
                #name::set_distinct_attribute(&index, client).await?;
                #name::set_primary_key(client).await?;

                Ok(())
            }

            /// Get the index if it exists, otherwise create it
            ///
            /// Arguments:
            /// - `client`: The MeiliSearch client
            pub async fn get_index(client: &meilisearch_sdk::client::Client) -> Result<meilisearch_sdk::indexes::Index, meilisearch_sdk::errors::Error> {
                let index = client.get_index(#model_attr_name).await;
                match index {
                    Ok(index) => Ok(index),
                    Err(meilisearch_sdk::errors::Error::Meilisearch(error)) => {
                        if error.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound {
                            let task = client.create_index(#model_attr_name, Some(#pk_name)).await?;
                            task.wait_for_completion(client, None, None).await?;
                            let index = client.get_index(#model_attr_name).await?;
                            Ok(index)
                        } else {
                            // trickle down the error
                            Err(meilisearch_sdk::errors::Error::Meilisearch(error))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    };

    expanded.into()
}
