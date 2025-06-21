//! A custom derive collection macro for Database/MongoDB models

use proc_macro::TokenStream;
use syn::spanned::Spanned;

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
            db: DatabaseShared,
            #[doc = "The shared connection for the `"]
            #[doc = #model_name]
            #[doc = "` collection"]
            col: CollectionShared<#model_ident>,
        }

        impl #handler_ident {
            /// Create a new instance of the handler
            pub fn new(db: &DatabaseShared) -> Self {
                let typed_col = db.clone().collection::<#model_ident>(#model_ident::collection_name());
                Self {
                    db: ::std::sync::Arc::clone(db),
                    col: ::std::sync::Arc::new(typed_col),
                }
            }

            #[doc = "Get the shared connection for"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub fn get_collection(&self) -> CollectionShared<#model_ident> {
                ::std::sync::Arc::clone(&self.col)
            }

            #[doc = "Find all documents in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_all(&self) -> mongodb::error::Result<Vec<#model_ident>> {
                let mut cursor = self.col.find(mongodb::bson::doc! {}).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Find a document by [`mongodb::bson::oid::ObjectId`] in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by_oid(&self, id: &mongodb::bson::oid::ObjectId) -> mongodb::error::Result<Option<#model_ident>> {
                self.col.find_one(mongodb::bson::doc! { "_id": id }).await
            }

            #[doc = "Find a document by Id in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by_id(&self, id: &str) -> mongodb::error::Result<Option<#model_ident>> {
                self.col.find_one(mongodb::bson::doc! { "id": id }).await
            }

            #[doc = "Find document by a filter in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> mongodb::error::Result<Option<#model_ident>> {
                self.col.find_one(filter).await
            }

            #[doc = "Find all documents by a filter in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn find_all_by(
                &self,
                filter: mongodb::bson::Document,
            ) -> mongodb::error::Result<Vec<#model_ident>> {
                let mut cursor = self.col.find(filter).await?;
                let mut results = Vec::new();

                while let Some(result) = cursor.try_next().await? {
                    results.push(result);
                }

                Ok(results)
            }

            #[doc = "Insert a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn insert(&self, docs: &mut Vec<#model_ident>) -> mongodb::error::Result<()> {
                // Iterate over the documents and add the `_id` field if it's missing
                for doc in docs.iter_mut() {
                    if doc.id().is_none() {
                        doc.set_id(mongodb::bson::oid::ObjectId::new());
                    }
                }
                self.col.insert_many(docs).await?;
                Ok(())
            }

            #[doc = "Update a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn update(&self, doc: &#model_ident, filter: Option<mongodb::bson::Document>, update: mongodb::bson::Document) -> mongodb::error::Result<#model_ident> {
                if doc.id().is_none() {
                    return Err(mongodb::error::Error::custom("Document must have an `_id` to be updated"));
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

                match self.col.find_one_and_update(filter, update).with_options(options).await? {
                    Some(result) => Ok(result),
                    None => Err(mongodb::error::Error::custom("Failed to update document")),
                }
            }

            /// Internally used to save a document
            async fn internal_save(&self, doc: &mut #model_ident, filter: Option<mongodb::bson::Document>) -> mongodb::error::Result<()> {
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

                match self.col.find_one_and_replace(filter, &(*doc)).with_options(options).await? {
                    Some(result) => {
                        match mongodb::bson::to_bson(&result)? {
                            mongodb::bson::Bson::Document(dd) => {
                                if id_needs_update {
                                    let resp_id = dd.get_object_id("_id").map_err(|e| {
                                        mongodb::error::Error::custom(format!(
                                            "Failed to get the current `_id` from document: {}",
                                            e
                                        ))
                                    })?;
                                    doc.set_id(resp_id);
                                };
                                Ok(())
                            }
                            _ => Err(mongodb::error::Error::custom("Failed to convert document into bson object")),
                        }
                    }
                    None => Err(mongodb::error::Error::custom("Failed to save document")),
                }
            }

            #[doc = "Save a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn save(&self, doc: &mut #model_ident, filter: Option<mongodb::bson::Document>) -> mongodb::error::Result<()> {
                doc.updated();
                self.internal_save(doc, filter).await
            }

            #[doc = "Save a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection without updating the `updated` field"]
            pub async fn save_direct(&self, doc: &mut #model_ident, filter: Option<mongodb::bson::Document>) -> mongodb::error::Result<()> {
                self.internal_save(doc, filter).await
            }

            #[doc = "Delete a document in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn delete(&self, doc: &#model_ident) -> mongodb::error::Result<mongodb::results::DeleteResult> {
                if doc.id().is_none() {
                    return Err(mongodb::error::Error::custom("Document must have an `_id` to be deleted"));
                }

                let filter = mongodb::bson::doc! { "_id": doc.id().unwrap() };
                self.col.delete_one(filter).await
            }

            #[doc = "Delete documents in the"]
            #[doc = #model_name_full]
            #[doc = "collection by filter"]
            pub async fn delete_by(&self, filter: mongodb::bson::Document) -> mongodb::error::Result<mongodb::results::DeleteResult> {
                self.col.delete_one(filter).await
            }

            #[doc = "Delete all documents in the"]
            #[doc = #model_name_full]
            #[doc = "collection"]
            pub async fn delete_all(&self) -> mongodb::error::Result<()> {
                let col = self.col.clone();
                let deref_col = ::std::sync::Arc::try_unwrap(col).unwrap_or_else(|arc| (*arc).clone());
                deref_col.drop().await
            }
        }
    };

    tokens.into()
}
