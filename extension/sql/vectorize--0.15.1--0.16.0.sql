-- src/api.rs:158
-- vectorize::api::generate
CREATE  FUNCTION vectorize."generate"(
	"input" TEXT, /* &str */
	"model" TEXT DEFAULT 'openai/gpt-3.5-turbo', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'generate_wrapper';

-- src/api.rs:168
-- vectorize::api::env_interpolate_guc
CREATE  FUNCTION vectorize."env_interpolate_guc"(
	"guc_name" TEXT /* &str */
) RETURNS TEXT /* core::result::Result<alloc::string::String, anyhow::Error> */
STRICT
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'env_interpolate_guc_wrapper';

-- src/api.rs:79
-- vectorize::api::encode
CREATE  FUNCTION vectorize."encode"(
	"input" TEXT, /* &str */
	"model" TEXT DEFAULT 'openai/text-embedding-ada-002', /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS double precision[] /* core::result::Result<alloc::vec::Vec<f64>, anyhow::Error> */
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'encode_wrapper';
