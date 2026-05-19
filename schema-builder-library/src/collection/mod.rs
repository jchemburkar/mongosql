/**
 * This module contains functionality for processing MongoDB collections
 * and how we operate with them.
 */
use crate::{
    DataService, Error, Result, data_service::CollectionInfo,
    schema::initial_schema::InitialSchema,
};

pub(crate) mod patterns;
use bson::doc;
use futures::TryStreamExt as _;
use mongosql::schema::Schema;
use std::{
    collections::HashMap,
    sync::{LazyLock, OnceLock},
};
use tracing::instrument;

#[cfg(test)]
mod test;

static EXCLUDE_DUNDERSCORE_PATTERN: LazyLock<glob::Pattern> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    glob::Pattern::new("__*")
        .expect("Internal error: `__*` could not be converted into a `glob::Pattern`")
});

static INCLUDE_LIST_IN_DB_AND_COLL_PAIRS: OnceLock<Vec<(String, String)>> = OnceLock::new();

/// DatabaseCollections is responsible for extracting the collections and views
/// and preparing them for processing.
#[derive(Debug, Default)]
pub struct DatabaseCollections {
    pub db: String,
    pub views: Vec<CollectionInfo>,
    pub collections: Vec<CollectionInfo>,
    pub timeseries: Vec<CollectionInfo>,
}

#[instrument(level = "trace", skip_all)]
pub async fn query_for_initial_schemas<S: DataService>(
    service: &S,
    db: &str,
    schema_collection: &str,
) -> Result<HashMap<String, Schema>, S::Error> {
    let mut initial_collection_schemas = HashMap::new();
    let mut cursor = service
        .find(db, schema_collection, doc! {})
        .await
        .map_err(Error::DataServiceError)?;

    // Try to parse all of the initial schemas, failing if any of them fail to
    // parse correctly
    while let Some(doc) = cursor.try_next().await.map_err(Error::DataServiceError)? {
        // Convert the Doc into our InitialSchema struct
        let InitialSchema { collection, schema } = InitialSchema::try_from(doc)
            .map_err(|_| Error::InitialSchemaError(schema_collection.to_string()))?;

        // If conversion is successful, add the schema to the map
        initial_collection_schemas.insert(collection, schema);
    }

    Ok(initial_collection_schemas)
}

impl DatabaseCollections {
    /// Create a new DatabaseCollections instance. Collections and Views within a database
    /// will be enumerated, checked for inclusion/exclusion, and prepared for
    /// processing. The caller must actually process the collections/views by calling
    /// their JoinHandle.
    #[instrument(
        name = "processing collections for database",
        level = "info",
        skip(service, include_list, exclude_list)
    )]
    pub async fn new<S: DataService>(
        service: &S,
        db: String,
        include_list: Vec<glob::Pattern>,
        exclude_list: Vec<glob::Pattern>,
    ) -> Result<Self, S::Error> {
        let collection_info = service
            .list_collections(&db)
            .await
            .map_err(Error::DataServiceError)?;

        DatabaseCollections::separate_collection_types(
            db,
            &include_list,
            &exclude_list,
            collection_info,
        )
        .await
        .map_err(Into::into)
    }
}
