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
}
