use crate::executor::ColumnJobParams;
use crate::openai::get_embeddings;
use crate::search::cosine_similarity_search;
use pgrx::prelude::*;

#[pg_extern]
fn search(
    job_name: &str,
    return_col: &str,
    query: &str,
    api_key: &str,
    num_results: default!(i32, 10),
) -> Result<Vec<pgrx::JsonB>, spi::Error> {
    // note: this is not the most performant implementation
    // this requires a query to metadata table to get the projects schema and table

    // TODO: simplify api signature as much as possible
    // get project metadata
    let _project_meta = get_vectorize_meta_spi(job_name).expect("metadata for project is missing");
    let project_meta: ColumnJobParams = serde_json::from_value(
        serde_json::to_value(_project_meta).expect("failed to deserialize metadata"),
    )
    .expect("failed to deserialize metadata");
    // TODO: this needs to introspect the project type to figure out where to get embeddings from
    // assuming default openai API for now
    // get embeddings
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let schema = project_meta.schema;
    let table = project_meta.table;
    // let return_col = project_meta.params.get("return_col").expect("no return_col");

    let embeddings =
        runtime.block_on(async { get_embeddings(&vec![query.to_string()], api_key).await });
    cosine_similarity_search(
        job_name,
        &schema,
        &table,
        return_col,
        num_results,
        &embeddings[0],
    )
}

fn get_vectorize_meta_spi(job_name: &str) -> Option<pgrx::JsonB> {
    // TODO: change to bind param
    let query = "
        SELECT params::jsonb
        FROM vectorize.vectorize_meta
        WHERE name = $1
    ";
    let r: Result<Option<pgrx::JsonB>, spi::Error> = Spi::get_one_with_args(
        &query,
        vec![(PgBuiltInOids::TEXTOID.oid(), job_name.into_datum())],
    );
    r.expect("failed to query vectorizie metadata table")
}
