use crate::guc;
use crate::init;
use crate::job::{create_event_trigger, create_trigger_handler, initalize_table_job};
use crate::transformers::http_handler::sync_get_model_info;
use crate::transformers::openai;
use crate::transformers::transform;
use crate::util;

use anyhow::{Context, Result};
use pgrx::prelude::*;
use vectorize_core::types::{self, TableMethod, VectorizeMeta};

#[allow(clippy::too_many_arguments)]
pub fn init_table(
    job_name: &str,
    schema: &str,
    table: &str,
    columns: Vec<String>,
    primary_key: &str,
    args: Option<serde_json::Value>,
    update_col: Option<String>,
    index_dist_type: types::IndexDist,
    transformer: &str,
    // search_alg is now deprecated
    search_alg: types::SimilarityAlg,
    table_method: types::TableMethod,
    // cron-like for a cron based update model, or 'realtime' for a trigger-based
    schedule: &str,
) -> Result<String> {
    let job_type = types::JobType::Columns;

    // validate table method
    // realtime is only compatible with the join method
    if schedule == "realtime" && table_method != TableMethod::join {
        error!("realtime schedule is only compatible with the join table method");
    }

    let arguments = match serde_json::to_value(args) {
        Ok(a) => a,
        Err(e) => {
            error!("invalid json for argument `args`: {}", e);
        }
    };
    let api_key = match arguments.get("api_key") {
        Some(k) => Some(serde_json::from_value::<String>(k.clone())?),
        None => None,
    };

    // get prim key type
    let pkey_type = init::get_column_datatype(schema, table, primary_key)?;
    init::init_pgmq()?;

    // certain embedding services require an API key, e.g. openAI
    // key can be set in a GUC, so if its required but not provided in args, and not in GUC, error
    match transformer {
        "text-embedding-ada-002" => {
            let openai_key = match api_key.clone() {
                Some(k) => k,
                None => match guc::get_guc(guc::VectorizeGuc::OpenAIKey) {
                    Some(k) => k,
                    None => {
                        error!("failed to get API key from GUC");
                    }
                },
            };
            openai::validate_api_key(&openai_key)?;
        }
        t => {
            // make sure transformer exists
            let _ = sync_get_model_info(t, api_key.clone()).context("transformer does not exist");
        }
    }

    let valid_params = types::JobParams {
        schema: schema.to_string(),
        table: table.to_string(),
        columns: columns.clone(),
        update_time_col: update_col,
        table_method: table_method.clone(),
        primary_key: primary_key.to_string(),
        pkey_type,
        api_key: api_key.clone(),
        schedule: schedule.to_string(),
    };
    let params =
        pgrx::JsonB(serde_json::to_value(valid_params.clone()).expect("error serializing params"));

    // write job to table
    let init_job_q = init::init_job_query();
    // using SPI here because it is unlikely that this code will be run anywhere but inside the extension.
    // background worker will likely be moved to an external container or service in near future
    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        match c.update(
            &init_job_q,
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), job_name.into_datum()),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    job_type.to_string().into_datum(),
                ),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    index_dist_type.to_string().into_datum(),
                ),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    transformer.to_string().into_datum(),
                ),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    // search_alg is now deprecated
                    search_alg.to_string().into_datum(),
                ),
                (PgBuiltInOids::JSONBOID.oid(), params.into_datum()),
            ]),
        ) {
            Ok(_) => (),
            Err(e) => {
                error!("error creating job: {}", e);
            }
        }
        Ok(())
    });
    if ran.is_err() {
        error!("error creating job");
    }
    let init_embed_q = init::init_embedding_table_query(job_name, transformer, &valid_params);

    let ran: Result<_, spi::Error> = Spi::connect(|mut c| {
        for q in init_embed_q {
            let _r = c.update(&q, None, None)?;
        }
        Ok(())
    });
    if let Err(e) = ran {
        error!("error creating embedding table: {}", e);
    }
    match schedule {
        "realtime" => {
            // setup triggers
            // create the trigger if not exists
            let trigger_handler = create_trigger_handler(job_name, &columns, primary_key);
            let insert_trigger = create_event_trigger(job_name, schema, table, "INSERT");
            let update_trigger = create_event_trigger(job_name, schema, table, "UPDATE");
            log!("insert trigger: {}", insert_trigger);
            log!("update trigger: {}", update_trigger);
            let _: Result<_, spi::Error> = Spi::connect(|mut c| {
                let _r = c.update(&trigger_handler, None, None)?;
                let _r = c.update(&insert_trigger, None, None)?;
                let _r = c.update(&update_trigger, None, None)?;
                Ok(())
            });
        }
        _ => {
            // initialize cron
            init::init_cron(schedule, job_name)?;
            log!("Initialized cron job");
        }
    }
    // start with initial batch load
    initalize_table_job(
        job_name,
        &valid_params,
        &job_type,
        index_dist_type,
        transformer,
        // search_alg is now deprecated
        search_alg,
    )?;
    Ok(format!("Successfully created job: {job_name}"))
}

pub fn search(
    job_name: &str,
    query: &str,
    api_key: Option<String>,
    return_columns: Vec<String>,
    num_results: i32,
    where_clause: Option<String>,
) -> Result<Vec<pgrx::JsonB>> {
    let project_meta: VectorizeMeta = util::get_vectorize_meta_spi(job_name)?;
    let proj_params: types::JobParams = serde_json::from_value(
        serde_json::to_value(project_meta.params).unwrap_or_else(|e| {
            error!("failed to serialize metadata: {}", e);
        }),
    )
    .unwrap_or_else(|e| error!("failed to deserialize metadata: {}", e));

    let proj_api_key = match api_key {
        // if api passed in the function call, use that
        Some(k) => Some(k),
        // if not, use the one from the project metadata
        None => proj_params.api_key.clone(),
    };

    let embeddings = transform(query, &project_meta.transformer, proj_api_key);

    match project_meta.index_dist_type {
        types::IndexDist::pgv_hnsw_l2 => l2_similarity_search(
            job_name,
            &proj_params,
            &return_columns,
            num_results,
            &embeddings[0],
            where_clause,
        ),
        types::IndexDist::pgv_hnsw_ip => ip_similarity_search(
            // Assuming a function for inner product search
            job_name,
            &proj_params,
            &return_columns,
            num_results,
            &embeddings[0],
            where_clause,
        ),
        types::IndexDist::pgv_hnsw_cosin => cosine_similarity_search(
            job_name,
            &proj_params,
            &return_columns,
            num_results,
            &embeddings[0],
            where_clause,
        ),
    }

    // search_alg is now deprecated
    match project_meta.search_alg {
        types::SimilarityAlg::pgv_cosine_similarity => cosine_similarity_search(
            job_name,
            &proj_params,
            &return_columns,
            num_results,
            &embeddings[0],
            where_clause,
        ),
    }
}

pub fn cosine_similarity_search(
    project: &str,
    job_params: &types::JobParams,
    return_columns: &[String],
    num_results: i32,
    embeddings: &[f64],
    where_clause: Option<String>,
) -> Result<Vec<pgrx::JsonB>> {
    let schema = job_params.schema.clone();
    let table = job_params.table.clone();

    // switch on table method
    let query = match job_params.table_method {
        TableMethod::append => single_table_cosine_similarity(
            project,
            &schema,
            &table,
            return_columns,
            num_results,
            where_clause,
        ),
        TableMethod::join => join_table_cosine_similarity(
            project,
            job_params,
            return_columns,
            num_results,
            where_clause,
        ),
    };
    log!("query: {}", query);
    Spi::connect(|client| {
        // let mut results: Vec<(pgrx::JsonB,)> = Vec::new();
        let mut results: Vec<pgrx::JsonB> = Vec::new();
        let tup_table = client.select(
            &query,
            None,
            Some(vec![(
                PgBuiltInOids::FLOAT8ARRAYOID.oid(),
                embeddings.into_datum(),
            )]),
        )?;
        for row in tup_table {
            match row["results"].value()? {
                Some(r) => results.push(r),
                None => error!("failed to get results"),
            }
        }
        Ok(results)
    })
}

fn join_table_cosine_similarity(
    project: &str,
    job_params: &types::JobParams,
    return_columns: &[String],
    num_results: i32,
    where_clause: Option<String>,
) -> String {
    let schema = job_params.schema.clone();
    let table = job_params.table.clone();
    let join_key = &job_params.primary_key;
    let cols = &return_columns
        .iter()
        .map(|s| format!("t0.{}", s))
        .collect::<Vec<_>>()
        .join(",");

    let where_str = if let Some(w) = where_clause {
        prepare_filter(&w, join_key)
    } else {
        "".to_string()
    };
    let inner_query = format!(
        "
    SELECT
        {join_key},
        1 - (embeddings <=> $1::vector) AS similarity_score
    FROM vectorize._embeddings_{project}
    ORDER BY similarity_score DESC
    "
    );
    format!(
        "
    SELECT to_jsonb(t) as results
    FROM (
        SELECT {cols}, t1.similarity_score
        FROM
            (
                {inner_query}
            ) t1
        INNER JOIN {schema}.{table} t0 on t0.{join_key} = t1.{join_key}
        {where_str}
    ) t
    ORDER BY t.similarity_score DESC
    LIMIT {num_results};
    "
    )
}

fn single_table_cosine_similarity(
    project: &str,
    schema: &str,
    table: &str,
    return_columns: &[String],
    num_results: i32,
    where_clause: Option<String>,
) -> String {
    let where_str = if let Some(w) = where_clause {
        format!("AND {}", w)
    } else {
        "".to_string()
    };
    format!(
        "
    SELECT to_jsonb(t) as results
    FROM (
        SELECT 
        1 - ({project}_embeddings <=> $1::vector) AS similarity_score,
        {cols}
    FROM {schema}.{table}
    WHERE {project}_updated_at is NOT NULL
    {where_str}
    ORDER BY similarity_score DESC
    LIMIT {num_results}
    ) t
    ",
        cols = return_columns.join(", "),
    )
}

// transform user's where_sql into the format search query expects
fn prepare_filter(filter: &str, pkey: &str) -> String {
    let wc = filter.replace(pkey, &format!("t0.{}", pkey));
    format!("AND {wc}")
}
