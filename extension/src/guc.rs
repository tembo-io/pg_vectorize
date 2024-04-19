use core::ffi::CStr;
use pgrx::*;

use anyhow::Result;

pub static VECTORIZE_HOST: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static VECTORIZE_DATABASE_NAME: GucSetting<Option<&CStr>> =
    GucSetting::<Option<&CStr>>::new(None);
pub static OPENAI_KEY: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static BATCH_SIZE: GucSetting<i32> = GucSetting::<i32>::new(10000);
pub static NUM_BGW_PROC: GucSetting<i32> = GucSetting::<i32>::new(1);
pub static EMBEDDING_SERVICE_HOST: GucSetting<Option<&CStr>> =
    GucSetting::<Option<&CStr>>::new(None);
pub static EMBEDDING_REQ_TIMEOUT_SEC: GucSetting<i32> = GucSetting::<i32>::new(120);
pub static OLLAMA_SERVICE_HOST: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);

// initialize GUCs
pub fn init_guc() {
    GucRegistry::define_string_guc(
        "vectorize.host",
        "unix socket url for Postgres",
        "unix socket path to the Postgres instance. Optional. Can also be set in environment variable.",
        &VECTORIZE_HOST,
        GucContext::Suset, GucFlags::default()
    );

    GucRegistry::define_string_guc(
        "vectorize.database_name",
        "Target database for vectorize operations",
        "Specifies the target database for vectorize operations.",
        &VECTORIZE_DATABASE_NAME,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        "vectorize.openai_key",
        "API key from OpenAI",
        "API key from OpenAI. Optional. Overridden by any values provided in function calls.",
        &OPENAI_KEY,
        GucContext::Suset,
        GucFlags::SUPERUSER_ONLY,
    );

    GucRegistry::define_string_guc(
        "vectorize.ollama_service_url",
        "Ollama server url",
        "Scheme, host, and port of the Ollama server",
        &OLLAMA_SERVICE_HOST,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_int_guc(
        "vectorize.batch_size",
        "Vectorize job batch size",
        "Number of records that can be included in a single vectorize job.",
        &BATCH_SIZE,
        1,
        100000,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        "vectorize.embedding_service_url",
        "Url for an OpenAI compatible embedding service",
        "Url to a service with request and response schema consistent with OpenAI's embeddings API.",
        &EMBEDDING_SERVICE_HOST,
        GucContext::Suset, GucFlags::default());

    GucRegistry::define_int_guc(
        "vectorize.num_bgw_proc",
        "Number of bgw processes",
        "Number of parallel background worker processes to run. Default is 1.",
        &NUM_BGW_PROC,
        1,
        10,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_int_guc(
        "vectorize.embedding_req_timeout_sec",
        "Timeout, in seconds, for embedding transform requests",
        "Number of seconds to wait for an embedding http request to complete. Default is 120 seconds.",
        &EMBEDDING_REQ_TIMEOUT_SEC,
        1,
        1800,
        GucContext::Suset,
        GucFlags::default(),
    );
}

// for handling of GUCs that can be error prone
#[derive(Debug)]
pub enum VectorizeGuc {
    Host,
    DatabaseName,
    OpenAIKey,
    EmbeddingServiceUrl,
    OllamaServiceUrl,
}

/// a convenience function to get this project's GUCs
pub fn get_guc(guc: VectorizeGuc) -> Option<String> {
    let val = match guc {
        VectorizeGuc::Host => VECTORIZE_HOST.get(),
        VectorizeGuc::DatabaseName => VECTORIZE_DATABASE_NAME.get(),
        VectorizeGuc::OpenAIKey => OPENAI_KEY.get(),
        VectorizeGuc::EmbeddingServiceUrl => EMBEDDING_SERVICE_HOST.get(),
        VectorizeGuc::OllamaServiceUrl => OLLAMA_SERVICE_HOST.get(),
    };
    if let Some(cstr) = val {
        if let Ok(s) = handle_cstr(cstr) {
            Some(s)
        } else {
            error!("failed to convert CStr to str");
        }
    } else {
        info!("no value set for GUC: {:?}", guc);
        None
    }
}

#[allow(dead_code)]
fn handle_cstr(cstr: &CStr) -> Result<String> {
    if let Ok(s) = cstr.to_str() {
        Ok(s.to_owned())
    } else {
        Err(anyhow::anyhow!("failed to convert CStr to str"))
    }
}
