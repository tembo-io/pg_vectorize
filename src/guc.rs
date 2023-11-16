use core::ffi::CStr;
use pgrx::*;

pub static VECTORIZE_HOST: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static OPENAI_KEY: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);

// initialize GUCs
pub fn init_guc() {
    GucRegistry::define_string_guc(
        "vectorize.host",
        "unix socket url for Postgres",
        "unix socket path to the Postgres instance. Optional. Can also be set in environment variable.",
        &VECTORIZE_HOST,
        GucContext::Suset, GucFlags::default());

    GucRegistry::define_string_guc(
        "vectorize.openai_key",
        "API key from OpenAI",
        "API key from OpenAI. Optional. Overridden by any values provided in function calls.",
        &OPENAI_KEY,
        GucContext::Suset,
        GucFlags::SUPERUSER_ONLY,
    );
}
