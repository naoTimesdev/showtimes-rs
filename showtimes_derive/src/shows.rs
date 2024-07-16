use proc_macro::TokenStream;
use syn::{spanned::Spanned, Lit};

pub(crate) fn expand_showmodel(ast: &syn::DeriveInput) -> TokenStream {
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
                                false
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

pub(crate) struct CreateHandler {
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

pub(crate) fn expand_handler(input: &CreateHandler) -> TokenStream {
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

    let model_name_full = format!("[`m::{}`]", model_name);

    // Generate the implementation of the trait
    let tokens = quote::quote! {
        #[derive(Debug, Clone)]
        #[doc = "A handler for the"]
        #[doc = #model_name_full]
        #[doc = "collection"]
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

            #[doc = "Find all documents in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_all(&self) -> anyhow::Result<Vec<#model_ident>> {
                let col = self.col.lock().await;
                let mut cursor = col.find(mongodb::bson::doc! {}).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Find a document by [`mongodb::bson::oid::ObjectId`] in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by_oid(&self, id: &mongodb::bson::oid::ObjectId) -> anyhow::Result<Option<#model_ident>> {
                let col = self.col.lock().await;
                let result = col.find_one(mongodb::bson::doc! { "_id": id }).await?;
                Ok(result)
            }

            #[doc = "Find a document by Id in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<#model_ident>> {
                let col = self.col.lock().await;
                let result = col.find_one(mongodb::bson::doc! { "id": id }).await?;
                Ok(result)
            }

            #[doc = "Find document by a filter in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Option<#model_ident>> {
                let col = self.col.lock().await;
                let result = col.find_one(filter).await?;
                Ok(result)
            }

            #[doc = "Find all documents by a filter in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_all_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> anyhow::Result<Vec<#model_ident>> {
                let col = self.col.lock().await;
                let mut cursor = col.find(filter).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Insert a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn insert(&self, docs: &mut Vec<#model_ident>) -> anyhow::Result<()> {
                let col = self.col.lock().await;
                // Iterate over the documents and add the `_id` field if it's missing
                for doc in docs.iter_mut() {
                    if doc.id().is_none() {
                        doc.set_id(mongodb::bson::oid::ObjectId::new());
                    }
                }
                col.insert_many(docs).await?;
                Ok(())
            }

            #[doc = "Update a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn update(&self, doc: &#model_ident, filter: Option<mongodb::bson::Document>, update: mongodb::bson::Document) -> anyhow::Result<#model_ident> {
                let col = self.col.lock().await;
                if doc.id().is_none() {
                    anyhow::bail!("Document must have an `_id` to be updated");
                }

                let filter = match filter {
                    Some(mut docf) => {
                        docf.insert("_id", doc.id());
                        docf
                    },
                    None => mongodb::bson::doc! { "_id": doc.id().unwrap() },
                };

                let mut options = mongodb::options::FindOneAndUpdateOptions::default();
                options.upsert = Some(true);
                let mut wc = mongodb::options::WriteConcern::default();
                wc.journal = Some(true);
                options.write_concern = Some(wc);

                match col.find_one_and_update(filter, update).with_options(options).await? {
                    Some(result) => Ok(result),
                    None => anyhow::bail!("Failed to update document"),
                }
            }

            #[doc = "Save a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn save(&self, doc: &mut #model_ident, filter: Option<mongodb::bson::Document>) -> anyhow::Result<()> {
                let col = self.col.lock().await;

                let mut wc = mongodb::options::WriteConcern::default();
                wc.journal = Some(true);

                let mut id_needs_update = false;
                let filter = match (doc.id(), filter) {
                    (Some(id), _) => mongodb::bson::doc! {"_id": id},
                    (None, None) => {
                        let new_id = mongodb::bson::oid::ObjectId::new();
                        doc.set_id(new_id);
                        mongodb::bson::doc! {"_id": new_id}
                    }
                    (None, Some(filter)) => {
                        id_needs_update = true;
                        filter
                    }
                };

                let mut options = mongodb::options::FindOneAndReplaceOptions::builder()
                    .upsert(Some(true))
                    .write_concern(Some(wc))
                    .return_document(Some(mongodb::options::ReturnDocument::After))
                    .build();

                match col.find_one_and_replace(filter, &(*doc)).with_options(options).await? {
                    Some(result) => {
                        match mongodb::bson::to_bson(&result)? {
                            mongodb::bson::Bson::Document(dd) => {
                                if id_needs_update {
                                    let resp_id = dd.get_object_id("_id")?;
                                    doc.set_id(resp_id);
                                };
                                Ok(())
                            }
                            _ => anyhow::bail!("Failed to convert document into bson object")
                        }
                    }
                    None => anyhow::bail!("Failed to save document"),
                }
            }

            #[doc = "Delete a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn delete(&self, doc: &#model_ident) -> anyhow::Result<mongodb::results::DeleteResult> {
                let col = self.col.lock().await;
                if doc.id().is_none() {
                    anyhow::bail!("Document must have an `_id` to be deleted");
                }

                let filter = mongodb::bson::doc! { "_id": doc.id().unwrap() };
                Ok(col.delete_one(filter).await?)
            }

            #[doc = "Delete documents in the"]
            #[doc = #model_name_full]
            #[doc = "collection by filter"]
            pub async fn delete_by(&self, filter: mongodb::bson::Document) -> anyhow::Result<mongodb::results::DeleteResult> {
                let col = self.col.lock().await;
                Ok(col.delete_one(filter).await?)
            }
        }
    };

    tokens.into()
}
