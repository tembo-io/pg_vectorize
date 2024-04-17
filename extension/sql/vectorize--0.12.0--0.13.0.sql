DROP FUNCTION vectorize."search";

-- src/api.rs:41
-- vectorize::api::search
CREATE  FUNCTION vectorize."search"(
	"job_name" TEXT, /* alloc::string::String */
	"query" TEXT, /* alloc::string::String */
	"api_key" TEXT DEFAULT NULL, /* core::option::Option<alloc::string::String> */
	"return_columns" TEXT[] DEFAULT ARRAY['*']::text[], /* alloc::vec::Vec<alloc::string::String> */
	"num_results" INT DEFAULT 10, /* i32 */
	"where_sql" TEXT DEFAULT NULL /* core::option::Option<alloc::string::String> */
) RETURNS TABLE (
	"search_results" jsonb  /* pgrx::datum::json::JsonB */
)
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'search_wrapper';
