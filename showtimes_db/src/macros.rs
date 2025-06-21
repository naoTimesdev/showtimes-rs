mod _hidden_macros {
    /// Quickly implement the [`crate::ShowModelHandler`] trait for a struct.
    #[doc(hidden)]
    #[macro_export]
    macro_rules! impl_trait_model {
        ($struct:ident, $name:literal, $id:ident, $updated:ident) => {
            impl ShowModelHandler for $struct {
                fn id(&self) -> Option<mongodb::bson::oid::ObjectId> {
                    self.$id.clone()
                }

                fn set_id(&mut self, id: mongodb::bson::oid::ObjectId) {
                    self.$id = Some(id);
                }

                fn unset_id(&mut self) {
                    self.$id = None;
                }

                fn collection_name() -> &'static str {
                    $name
                }

                fn updated(&mut self) {
                    self.$updated = ::jiff::Timestamp::now();
                }
            }
        };
        ($struct:ident, $name:literal, $id:ident) => {
            impl ShowModelHandler for $struct {
                fn id(&self) -> Option<mongodb::bson::oid::ObjectId> {
                    self.$id.clone()
                }

                fn set_id(&mut self, id: mongodb::bson::oid::ObjectId) {
                    self.$id = Some(id);
                }

                fn unset_id(&mut self) {
                    self.$id = None;
                }

                fn collection_name() -> &'static str {
                    $name
                }

                fn updated(&mut self) {
                    // Ignored
                }
            }
        };
    }

    /// Implement the handler for the models
    #[doc(hidden)]
    #[macro_export]
    macro_rules! impl_handler_model {
        ($model:ty, $handler:ident, $col_name:literal) => {
            #[derive(Debug, Clone)]
            #[doc = "A handler for the [`"]
            #[doc = $col_name]
            #[doc = "`] collection"]
            pub struct $handler {
                /// The shared database connection
                #[expect(dead_code)]
                db: DatabaseShared,
                #[doc = "The shared connection for the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                col: CollectionShared<$model>,
            }

            impl $handler {
                /// Create a new instance of the handler
                pub fn new(db: &DatabaseShared) -> Self {
                    let typed_col = db.clone().collection::<$model>(<$model>::collection_name());
                    Self {
                        db: ::std::sync::Arc::clone(db),
                        col: ::std::sync::Arc::new(typed_col),
                    }
                }

                #[doc = "Get the shared connection for [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub fn get_collection(&self) -> CollectionShared<$model> {
                    ::std::sync::Arc::clone(&self.col)
                }

                #[doc = "Find all documents in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn find_all(&self) -> mongodb::error::Result<Vec<$model>> {
                    let mut cursor = self.col.find(mongodb::bson::doc! {}).await?;
                    let mut results = Vec::new();

                    while let Some(result) = cursor.try_next().await? {
                        results.push(result);
                    }

                    Ok(results)
                }

                #[doc = "Find a document by [`mongodb::bson::oid::ObjectId`] in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn find_by_oid(
                    &self,
                    id: &mongodb::bson::oid::ObjectId,
                ) -> mongodb::error::Result<Option<$model>> {
                    self.col.find_one(mongodb::bson::doc! { "_id": id }).await
                }

                #[doc = "Find a document by Id in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn find_by_id(&self, id: &str) -> mongodb::error::Result<Option<$model>> {
                    self.col.find_one(mongodb::bson::doc! { "id": id }).await
                }

                #[doc = "Find document by a filter in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn find_by(
                    &self,
                    filter: mongodb::bson::Document,
                ) -> mongodb::error::Result<Option<$model>> {
                    self.col.find_one(filter).await
                }

                #[doc = "Find all documents by a filter in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn find_all_by(
                    &self,
                    filter: mongodb::bson::Document,
                ) -> mongodb::error::Result<Vec<$model>> {
                    let mut cursor = self.col.find(filter).await?;
                    let mut results = Vec::new();

                    while let Some(result) = cursor.try_next().await? {
                        results.push(result);
                    }

                    Ok(results)
                }

                #[doc = "Insert a document in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn insert(&self, docs: &mut Vec<$model>) -> mongodb::error::Result<()> {
                    // Iterate over the documents and add the `_id` field if it's missing
                    for doc in docs.iter_mut() {
                        if doc.id().is_none() {
                            doc.set_id(mongodb::bson::oid::ObjectId::new());
                        }
                    }
                    self.col.insert_many(docs).await?;
                    Ok(())
                }

                #[doc = "Update a document in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn update(
                    &self,
                    doc: &$model,
                    filter: Option<mongodb::bson::Document>,
                    update: mongodb::bson::Document,
                ) -> mongodb::error::Result<$model> {
                    if doc.id().is_none() {
                        return Err(mongodb::error::Error::custom(
                            "Document must have an `_id` to be updated",
                        ));
                    }

                    let filter = match filter {
                        Some(mut docf) => {
                            docf.insert("_id", doc.id());
                            docf
                        }
                        None => mongodb::bson::doc! { "_id": doc.id().unwrap() },
                    };

                    let mut options = mongodb::options::FindOneAndUpdateOptions::default();
                    options.upsert = Some(true);
                    let mut wc = mongodb::options::WriteConcern::default();
                    wc.journal = Some(true);
                    options.write_concern = Some(wc);

                    match self
                        .col
                        .find_one_and_update(filter, update)
                        .with_options(options)
                        .await?
                    {
                        Some(result) => Ok(result),
                        None => Err(mongodb::error::Error::custom("Failed to update document")),
                    }
                }

                /// Internally used to save a document
                async fn internal_save(
                    &self,
                    doc: &mut $model,
                    filter: Option<mongodb::bson::Document>,
                ) -> mongodb::error::Result<()> {
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

                    let options = mongodb::options::FindOneAndReplaceOptions::builder()
                        .upsert(Some(true))
                        .write_concern(Some(wc))
                        .return_document(Some(mongodb::options::ReturnDocument::After))
                        .build();

                    match self
                        .col
                        .find_one_and_replace(filter, &(*doc))
                        .with_options(options)
                        .await?
                    {
                        Some(result) => match mongodb::bson::to_bson(&result)? {
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
                            _ => Err(mongodb::error::Error::custom(
                                "Failed to convert document into bson object",
                            )),
                        },
                        None => Err(mongodb::error::Error::custom("Failed to save document")),
                    }
                }

                #[doc = "Save a document in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn save(
                    &self,
                    doc: &mut $model,
                    filter: Option<mongodb::bson::Document>,
                ) -> mongodb::error::Result<()> {
                    doc.updated();
                    self.internal_save(doc, filter).await
                }

                #[doc = "Save a document in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection without updating the `updated` field"]
                pub async fn save_direct(
                    &self,
                    doc: &mut $model,
                    filter: Option<mongodb::bson::Document>,
                ) -> mongodb::error::Result<()> {
                    self.internal_save(doc, filter).await
                }

                #[doc = "Delete a document in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn delete(
                    &self,
                    doc: &$model,
                ) -> mongodb::error::Result<mongodb::results::DeleteResult> {
                    if doc.id().is_none() {
                        return Err(mongodb::error::Error::custom(
                            "Document must have an `_id` to be deleted",
                        ));
                    }

                    let filter = mongodb::bson::doc! { "_id": doc.id().unwrap() };
                    self.col.delete_one(filter).await
                }

                #[doc = "Delete documents in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection by filter"]
                pub async fn delete_by(
                    &self,
                    filter: mongodb::bson::Document,
                ) -> mongodb::error::Result<mongodb::results::DeleteResult> {
                    self.col.delete_one(filter).await
                }

                #[doc = "Delete all documents in the [`"]
                #[doc = $col_name]
                #[doc = "`] collection"]
                pub async fn delete_all(&self) -> mongodb::error::Result<()> {
                    let col = self.col.clone();
                    let deref_col =
                        ::std::sync::Arc::try_unwrap(col).unwrap_or_else(|arc| (*arc).clone());
                    deref_col.drop().await
                }
            }
        };
    }
}
