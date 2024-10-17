pub fn init_table(
    job_name: &str,
    schema: &str,
    table: &str,
    columns: Vec<String>,
    primary_key: &str,
    update_col: Option<String>,
    index_dist_type: types::IndexDist,
    transformer: &Model,
    table_method: types::TableMethod,
    schedule: &str, // cron-like or 'realtime' for trigger-based updates
) -> Result<String> {
    if schedule == "realtime" && table_method != TableMethod::join {
        error!("realtime schedule is only compatible with the join table method");
    }

    let pkey_type = init::get_column_datatype(schema, table, primary_key)?;
    init::init_pgmq()?;

    let guc_configs = get_guc_configs(&transformer.source);
    let optional_args = match transformer.source {
        ModelSource::OpenAI => {
            openai::validate_api_key(
                &guc_configs
                    .api_key
                    .clone()
                    .context("OpenAI key is required")?,
            )?;
            None
        }
        ModelSource::Tembo => error!("Tembo not implemented for search yet"),
        ModelSource::Ollama => {
            let url = guc::get_guc(guc::VectorizeGuc::OllamaServiceUrl)
                .context("Failed to get Ollama URL from GUC")?;
            check_model_host(&url).context("Error with model host")?;
            None
        }
        ModelSource::Portkey => Some(serde_json::json!({
            "virtual_key": guc_configs
                .virtual_key
                .clone()
                .context("Portkey virtual key is required")?
        })),
        _ => None,
    };

    let provider = get_provider(
        &transformer.source,
        guc_configs.api_key.clone(),
        guc_configs.service_url.clone(),
        guc_configs.virtual_key.clone(),
    )?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .context("Failed to initialize tokio runtime")?;

    let model_dim = runtime
        .block_on(async { provider.model_dim(&transformer.api_name()).await })
        .context("Error getting model dimension")?;

    let valid_params = types::JobParams {
        schema: schema.to_string(),
        table: table.to_string(),
        columns: columns.clone(),
        update_time_col: update_col,
        table_method: table_method.clone(),
        primary_key: primary_key.to_string(),
        pkey_type,
        api_key: guc_configs.api_key.clone(),
        schedule: schedule.to_string(),
        args: optional_args,
    };

    let params =
        pgrx::JsonB(serde_json::to_value(&valid_params).context("Error serializing parameters")?);

    let init_job_q = init::init_job_query();
    Spi::connect(|mut c| {
        c.update(
            &init_job_q,
            None,
            Some(vec![
                (PgBuiltInOids::TEXTOID.oid(), job_name.into_datum()),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    index_dist_type.to_string().into_datum(),
                ),
                (
                    PgBuiltInOids::TEXTOID.oid(),
                    transformer.to_string().into_datum(),
                ),
                (PgBuiltInOids::JSONBOID.oid(), params.into_datum()),
            ]),
        )
        .context("Error creating job")
    })?;

    let init_embed_q =
        init::init_embedding_table_query(job_name, &valid_params, &index_dist_type, model_dim);
    Spi::connect(|mut c| {
        for q in init_embed_q {
            c.update(&q, None, None)
                .context("Error initializing embedding table")?;
        }
        Ok(())
    })?;

    match schedule {
        "realtime" => {
            let trigger_handler = create_trigger_handler(job_name, &columns, primary_key);
            let insert_trigger = create_event_trigger(job_name, schema, table, "INSERT");
            let update_trigger = create_event_trigger(job_name, schema, table, "UPDATE");
            Spi::connect(|mut c| {
                c.update(&trigger_handler, None, None)?;
                c.update(&insert_trigger, None, None)?;
                c.update(&update_trigger, None, None)?;
                Ok(())
            })?;
        }
        _ => init::init_cron(schedule, job_name).context("Error initializing cron job")?,
    }

    // start with initial batch load
    initalize_table_job(job_name, &valid_params, index_dist_type, transformer)?;
    Ok(format!("Successfully created job: {job_name}"))
}
