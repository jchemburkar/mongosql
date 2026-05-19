use std::sync::Arc;

use bson::{Document, doc};
use futures::TryStreamExt as _;
use mongosql::schema::Schema;
use schema_derivation::schema_for_document;
use tracing::{info, instrument, warn};

pub(crate) mod initial_schema;

use crate::{
    DataService, Error, Result, VIEW_SAMPLE_SIZE,
    data_service::CollectionInfo,
    partitioning::Partition,
};

#[derive(Debug, PartialEq, Clone)]
pub struct SinglePartition {
    pub partition: Partition,
    pub partition_key: String,
    pub hint: Option<Document>,
    pub partition_ix: usize,
}

pub const PARTITION_DOCS_PER_ITERATION: i64 = 20;

/// A utility function for deriving the schema for a single partition of a collection.
///
/// For each partition:
///  1. If there is an initial schema
///     The initial schema is used as the "seed" for the partition.
///
///  2. If there is no initial schema
///     An aggregation operation is issued that sorts based on the partition key and limits the
///     result set to 20 documents. The schema for these documents is computed and "seeds" the
///     schema for the partition.
///
/// The builder then enters a loop where it repeatedly issues query operations for documents that
/// do not match the computed schema. The schema for each resulting document is calculated and
/// merged with the existing schema, resulting in a new schema that meets all encountered documents
/// so far. This operation repeats until there are no more results within the partition.
#[instrument(level = "trace", skip_all)]
pub async fn derive_schema_for_partition<S: DataService>(
    service: &S,
    db: &str,
    collection: &str,
    initial_schema_doc: Option<Arc<Schema>>,
    single_partition: SinglePartition,
) -> Result<Schema, S::Error> {
    let mut ignored_ids = Vec::new();
    let mut partition = single_partition.partition;
    let partition_key = single_partition.partition_key.as_str();
    let hint = single_partition.hint;
    let partition_ix = single_partition.partition_ix;

    // The initial schema might be empty, in which case we
    // default to `Unsat` if there are also no refinement entries for the DB
    let mut schema = initial_schema_doc
        .map(|s| Schema::simplify(&s))
        .unwrap_or(Schema::Unsat);

    let mut saw_unstable = false;

    loop {
        info!(db, collection, "querying partition: {partition_ix}");

        // This is a somewhat expensive clone, but there isn't a try_from for
        // a schema reference :(
        let doc = (schema != Schema::Unsat)
            .then(|| bson::Document::try_from(schema.clone()))
            .transpose()?;

        let pipeline = vec![
            partition.generate_match(doc, &ignored_ids, partition_key),
            doc! { "$sort": {partition_key: 1}},
            doc! { "$limit": PARTITION_DOCS_PER_ITERATION },
        ];
        let mut cursor = service
            .aggregate(db, collection, pipeline, hint.clone())
            .await
            .map_err(Error::DataServiceError)?;

        let mut no_result = true;
        let mut iter_schema = Schema::Unsat;
        while let Some(doc) = cursor.try_next().await.map_err(Error::DataServiceError)? {
            info!(db, collection, "processing partition {partition_ix}");
            if let Some(id) = doc.get(partition_key) {
                partition.min = id.clone();
                let old_schema = iter_schema.clone();
                iter_schema = iter_schema.union(&schema_for_document(&doc));

                // There is a bug in Server where $jsonSchema operator don't work with empty keys.
                // To avoid getting caught in an infinite loop, we push to a list of ignored IDs in the event
                // empty keys exists in the partition.
                // See SERVER-92443 and https://github.com/10gen/schema-manager-rs/pull/754 for more context.

                if old_schema == iter_schema {
                    ignored_ids.push(id.clone());
                }
                no_result = false;
            } else {
                warn!(db, collection, "document {partition_key} field");
                continue;
            };
        }
        if no_result {
            break;
        }

        schema = schema.union(&iter_schema);

        // If the schema for this partition becomes unstable, we should do at most one more
        // iteration to see if we detect any additional properties. After two iterations with an
        // unstable schema, we should stop deriving schema for this partition since it is unlikely
        // new information will be added.

        if schema.is_unstable() {
            if saw_unstable {
                break;
            } else {
                saw_unstable = true;
            }
        }
    }

    Ok(schema)
}

/// derive_schema_for_view takes a CollectionInfo and executes the pipeline
/// against the viewOn collection to generate a schema for the view.
/// It does this by first prepending $sample to the pipeline
#[instrument(level = "trace", skip_all)]
pub async fn derive_schema_for_view<S: DataService>(
    service: &S,
    db: &str,
    view: &CollectionInfo,
) -> Option<Schema> {
    let pipeline = vec![doc! { "$sample": { "size": VIEW_SAMPLE_SIZE } }]
        .into_iter()
        .chain(view.options.pipeline.clone().into_iter())
        .collect::<Vec<Document>>();

    let mut cursor = service
        .aggregate(db, &view.options.view_on, pipeline, None)
        .await
        .inspect_err(|e| {
            warn!(
                db,
                view_name = view.name,
                "view sampling encountered an error: {e}"
            )
        })
        .ok()?;

    let mut schema = None;
    let mut iterations = 0u64;
    while let Some(Ok(doc)) = cursor.try_next().await.transpose() {
        // Notify every 100 iterations, so it isn't too spammy
        if iterations.is_multiple_of(100) {
            info!(
                db,
                view_name = view.name,
                iteration = iterations,
                "Sampling view"
            );
        }

        schema = schema.map_or(Some(schema_for_document(&doc)), |s: Schema| {
            Some(s.union(&schema_for_document(&doc)))
        });

        iterations += 1;
    }

    schema
}
