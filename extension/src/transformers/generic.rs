use anyhow::Result;

use crate::guc;
use vectorize_core::transformers::generic::{find_placeholders, interpolate};

pub fn get_generic_svc_url() -> Result<String> {
    if let Some(url) = guc::get_guc(guc::VectorizeGuc::EmbeddingServiceUrl) {
        if let Some(phs) = find_placeholders(&url) {
            let interpolated = interpolate(&url, phs)?;
            Ok(interpolated)
        } else {
            Ok(url)
        }
    } else {
        Err(anyhow::anyhow!("vectorize.embedding_service_url not set"))
    }
}
