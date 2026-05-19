use agg_ast::definitions::Namespace;
use mongosql::schema::Schema;
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

// DataService trait and implementations
pub mod data_service;
pub use data_service::{
    CollectionInfo, CollectionOptions, CollectionType, DataService, TimeSeriesOptions,
};

#[cfg(all(feature = "native-client", feature = "wasm"))]
compile_error!("`native-client` and `wasm` features are mutually exclusive");

#[cfg(feature = "native-client")]
pub use data_service::MongoDbDataService;

#[cfg(feature = "wasm")]
pub use data_service::{JsDataService, WasmDataService};

#[cfg(feature = "native-client")]
pub mod client_util;

mod consts;
pub use consts::{DISALLOWED_DB_NAMES, VIEW_SAMPLE_SIZE};
mod collection;
pub use collection::{DatabaseCollections, query_for_initial_schemas};
mod partitioning;
pub use partitioning::{PartitionedCollection, get_partitions};
mod errors;
mod schema;
pub use errors::Error;
pub use schema::{SinglePartition, derive_schema_for_partition, derive_schema_for_view};

/// Re-export of mongosql Schema type for convenience
pub type MongoSqlSchema = Schema;

pub type Result<T, E> = std::result::Result<T, Error<E>>;

/// A struct representing namespace information for a view or collection.
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceInfo {
    /// The name of the database.
    pub db_name: String,

    /// The name of the collection or view which this schema represents.
    pub coll_or_view_name: String,

    /// The type of namespace (collection or view).
    pub namespace_type: NamespaceType,
}

impl From<NamespaceInfo> for Namespace {
    fn from(value: NamespaceInfo) -> Self {
        Namespace {
            database: value.db_name,
            collection: value.coll_or_view_name,
        }
    }
}

/// A struct representing schema information for a specific namespace (a view
/// or collection).
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceInfoWithSchema {
    pub namespace_info: NamespaceInfo,

    /// The schema for the namespace.
    pub namespace_schema: Arc<Schema>,
}

/// An enum representing the two namespace types for which this library
/// can generate schema: Collection and View.
#[derive(Debug, PartialEq, Clone)]
pub enum NamespaceType {
    Collection,
    View,
}

impl Display for NamespaceType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            NamespaceType::Collection => write!(f, "Collection"),
            NamespaceType::View => write!(f, "View"),
        }
    }
}
