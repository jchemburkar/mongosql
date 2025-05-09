use itertools::Itertools;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Bson},
    sync::Client,
};
use std::{
    collections::BTreeMap,
    env,
    io::{self},
    string::ToString,
};
use thiserror::Error;

lazy_static! {
    pub static ref MONGODB_URI: String = format!(
        "mongodb://localhost:{}",
        env::var("MDB_TEST_LOCAL_PORT").unwrap_or_else(|_| "27017".to_string())
    );
}

pub mod index;
pub use index::*;
pub mod query;
pub use query::*;
pub mod schema_derivation;
pub use schema_derivation::*;
pub mod build_utils;
pub mod e2e_db_manager;

pub use build_utils::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read directory: {0:?}")]
    InvalidDirectory(io::Error),
    #[error("failed to load file paths: {0:?}")]
    InvalidFilePath(io::Error),
    #[error("failed to read file: {0:?}")]
    InvalidFile(io::Error),
    #[error("unable to read file to string: {0:?}")]
    CannotReadFileToString(io::Error),
    #[error("unable to deserialize YAML file: {0:?}")]
    CannotDeserializeYaml((String, serde_yaml::Error)),
    #[error("failed to create mongodb client: {0:?}")]
    CannotCreateMongoDBClient(mongodb::error::Error),
    #[error("failed to drop db '{0}': {1:?}")]
    MongoDBDrop(String, mongodb::error::Error),
    #[error("failed to insert into '{0}.{1}': {2:?}")]
    MongoDBInsert(String, String, mongodb::error::Error),
    #[error("failed to create indexes for '{0}.{1}': {2:?}")]
    MongoDBCreateIndexes(String, String, mongodb::error::Error),
    #[error("failed to convert schema to MongoSql model: {0:?}")]
    InvalidSchema(mongosql::schema::Error),
    #[error("{0}")]
    UnsupportedBsonType(mongosql::schema::Error),
    #[error("failed to translate query: {0}")]
    Translation(mongosql::result::Error),
    #[error("failed to run aggregation: {0:?}")]
    MongoDBAggregation(mongodb::error::Error),
    #[error("failed to deserialize ExplainResult: {0:?}")]
    ExplainDeserialization(mongodb::bson::de::Error),
    #[error("invalid root stage: {0}")]
    InvalidRootStage(String),
    #[error("no queryPlanner found: {0:?}")]
    MissingQueryPlanner(ExplainResult),
    #[error("general mongodb error: {0:?}")]
    MongoDBErr(mongodb::error::Error),
}

impl From<mongosql::schema::Error> for Error {
    fn from(e: mongosql::schema::Error) -> Self {
        match e {
            mongosql::schema::Error::UnsupportedBsonType(_) => Error::UnsupportedBsonType(e),
            _ => Error::InvalidSchema(e),
        }
    }
}

/// load_catalog_data drops any existing catalog data and then inserts the
/// provided catalog data into the mongodb instance.
pub fn load_catalog_data(
    client: &Client,
    catalog_data: BTreeMap<String, BTreeMap<String, Vec<Bson>>>,
) -> Result<(), Error> {
    let catalog_dbs = catalog_data.keys().collect_vec();
    drop_catalog_data(client, catalog_dbs)?;

    for (db, coll_data) in catalog_data {
        let client_db = client.database(db.as_str());

        for (coll, documents) in coll_data {
            let client_coll = client_db.collection::<Bson>(coll.as_str());
            client_coll
                .insert_many(documents)
                .run()
                .map_err(|e| Error::MongoDBInsert(db.clone(), coll, e))?;
        }
    }

    Ok(())
}

/// drop_catalog_data drops all dbs in the provided list.
pub fn drop_catalog_data<T: Into<String>>(
    client: &Client,
    catalog_dbs: Vec<T>,
) -> Result<(), Error> {
    for db in catalog_dbs {
        let db = db.into();
        client
            .database(&db)
            .drop()
            .run()
            .map_err(|e| Error::MongoDBDrop(db.clone(), e))?;
    }
    Ok(())
}
