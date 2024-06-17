use proc_macro::TokenStream;
use syn::{spanned::Spanned, Lit};

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
    impl_db_model(&input)
}

fn impl_db_model(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    // run only on struct
    match &ast.data {
        syn::Data::Struct(data) => {
            // Check if the struct has a `_id` field with no public visibility
            // And also have the `#[serde(skip_serializing)]` attribute
            let id_field = data
                .fields
                .iter()
                .find(|&field| field.ident.as_ref().unwrap() == "_id");

            if let Some(id_field) = id_field {
                // Check visibility
                if id_field.vis != syn::Visibility::Inherited {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            id_field,
                            "The `_id` field must have private visibility",
                        )
                        .to_compile_error(),
                    );
                }

                // Check if the field has the `#[serde(skip_serializing_if = "Object::is_none")]` attribute
                let has_skip_serializing = id_field.attrs.iter().any(|attr| {
                    attr.path().is_ident("serde")
                        && attr.parse_args::<syn::Meta>().map_or(false, |meta| {
                            if let syn::Meta::NameValue(meta) = meta {
                                if let syn::Expr::Lit(lit) = &meta.value {
                                    if let syn::Lit::Str(litstr) = &lit.lit {
                                        return litstr.token().to_string() == "\"Option::is_none\""
                                            && meta.path.is_ident("skip_serializing_if");
                                    }
                                }
                                return false;
                            } else {
                                false
                            }
                        })
                });

                if !has_skip_serializing {
                    return TokenStream::from(
                        syn::Error::new_spanned(
                            id_field,
                            r#"The `_id` field must have the `#[serde(skip_serializing_if = "Object::is_none")]` attribute"#,
                        )
                        .to_compile_error(),
                    );
                }
            } else {
                return TokenStream::from(
                    syn::Error::new_spanned(name, "Missing required field: `_id`")
                        .to_compile_error(),
                );
            }
        }
        _ => {
            return TokenStream::from(
                syn::Error::new_spanned(
                    name,
                    "Expected a struct for the `ShowModelHandler` derive macro",
                )
                .to_compile_error(),
            );
        }
    }

    // Variable to hold the col_name value if found
    let mut col_name: Option<String> = None;

    // Look for the col_name attribute
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("col_name") {
            match attr.parse_args::<Lit>() {
                Ok(Lit::Str(lit_str)) => col_name = Some(lit_str.value()),
                _ => {
                    return TokenStream::from(
                        syn::Error::new_spanned(attr, "Expected a string literal for `col_name`")
                            .to_compile_error(),
                    )
                }
            }
        }
    }

    // Check if the required attributes were found, otherwise emit an error
    let col_name = match col_name {
        Some(value) => value,
        None => {
            return TokenStream::from(
                syn::Error::new_spanned(name, "Missing required attribute: `col_name`")
                    .to_compile_error(),
            )
        }
    };

    // Generate the implementation of the trait
    let expanded = quote::quote! {
        impl #name {
            pub fn id(&self) -> Option<mongodb::bson::oid::ObjectId> {
                self._id.clone()
            }

            pub fn set_id(&mut self, id: mongodb::bson::oid::ObjectId) {
                self._id = Some(id);
            }

            pub fn unset_id(&mut self) {
                self._id = None;
            }

            pub fn collection_name() -> &'static str {
                #col_name
            }
        }
    };

    expanded.into()
}

struct CreateHandler {
    name: syn::TypePath,
    // optional handler name override
    handler_name: Option<syn::Ident>,
}

impl syn::parse::Parse for CreateHandler {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let mut handler_name: Option<syn::Ident> = None;
        if input.parse::<syn::Token![,]>().is_ok() {
            if let Ok(ident) = input.parse() {
                handler_name = Some(ident);
            };
        }
        Ok(Self { name, handler_name })
    }
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
    let input = syn::parse_macro_input!(input as CreateHandler);

    // name is model name

    // handler name is model name with Handler suffix
    let model_ident = &input.name;
    let model_name = model_ident.path.segments.last().unwrap().ident.to_string();
    let (_, handler_ident) = match &input.handler_name {
        Some(ident) => (ident.to_string(), ident.clone()),
        None => {
            let handler_name = format!("{}Handler", model_name);
            let handler_ident = syn::Ident::new(&handler_name, model_ident.span());
            (handler_name, handler_ident)
        }
    };

    // Generate the implementation of the trait
    let tokens = quote::quote! {
        #[derive(Debug, Clone)]
        #[doc = "A handler for the `"]
        #[doc = #model_name]
        #[doc = "` collection"]
        pub struct #handler_ident {
            /// The shared database connection
            pub db: DatabaseMutex,
            #[doc = "The shared connection for the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub col: CollectionMutex<#model_ident>,
        }

        impl #handler_ident {
            /// Create a new instance of the handler
            pub async fn new(db: DatabaseMutex) -> Self {
                let typed_col = db.lock().await.collection::<#model_ident>(#model_ident::collection_name());
                Self {
                    db,
                    col: std::sync::Arc::new(tokio::sync::Mutex::new(typed_col)),
                }
            }

            #[doc = "Find all documents in the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub async fn find_all(&self) -> anyhow::Result<Vec<#model_ident>> {
                let col = self.col.lock().await;
                let mut cursor = col.find(None, None).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Find a document by its id in the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<#model_ident>> {
                let col = self.col.lock().await;
                let filter = mongodb::bson::doc! { "_id": id };
                let result = col.find_one(filter, None).await?;
                Ok(result)
            }

            #[doc = "Find document by a filter in the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub async fn find_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Option<#model_ident>> {
                let col = self.col.lock().await;
                let result = col.find_one(filter, None).await?;
                Ok(result)
            }

            #[doc = "Find all documents by a filter in the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub async fn find_all_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Vec<#model_ident>> {
                let col = self.col.lock().await;
                let mut cursor = col.find(filter, None).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Insert a document in the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            pub async fn insert(&self, docs: Vec<#model_ident>) -> anyhow::Result<()> {
                let col = self.col.lock().await;
                col.insert_many(docs, None).await?;
                Ok(())
            }

            // TODO: Add `update` or `upsert` method, `delete` method, etc.
            // TODO: A more complex query method can be done manually by using the `col` field
        }
    };

    tokens.into()
}
