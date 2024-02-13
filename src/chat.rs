use crate::executor::VectorizeMeta;
use crate::guc;
use crate::types;
use crate::util::get_vectorize_meta_spi;

use anyhow::{anyhow, Result};
use handlebars::Handlebars;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use pgrx::prelude::*;
use serde::Serialize;
use tiktoken_rs::{get_bpe_from_model, model::get_context_size, CoreBPE};

struct PromptTemplate {
    pub sys_prompt: String,
    pub user_prompt: String,
}

struct RenderedPromt {
    pub sys_rendered: String,
    pub user_rendered: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContextualSearch {
    pub record_id: String,
    pub content: String,
    pub token_ct: i32,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub context: Vec<ContextualSearch>,
    pub chat_response: String,
}

pub fn call_chat(
    agent_name: &str,
    query: &str,
    chat_model: &str,
    task: &str,
    api_key: Option<&str>,
    num_context: i32,
    force_trim: bool,
) -> Result<ChatResponse> {
    // get job metadata
    let project_meta: VectorizeMeta = if let Ok(Some(js)) = get_vectorize_meta_spi(agent_name) {
        js
    } else {
        error!("failed to get project metadata");
    };

    let job_params = serde_json::from_value::<types::JobParams>(project_meta.params.clone())
        .unwrap_or_else(|e| error!("failed to deserialize job params: {}", e));

    // for various token count estimations
    let bpe = get_bpe_from_model(chat_model).expect("failed to get BPE from model");

    // can only be 1 column in a chat job, for now, so safe to grab first element
    let content_column = job_params.columns[0].clone();
    let pk = job_params.primary_key;
    let columns = vec![pk.clone(), content_column.clone()];
    // query the relevant vectorize table using the query
    // TODO: refactor so we can call an internal access vector search function
    let search_results: Result<Vec<ContextualSearch>, spi::Error> = Spi::connect(|c| {
        let mut results: Vec<ContextualSearch> = Vec::new();
        let q = format!(
            "
        select search_results from vectorize.search(
            job_name => '{agent_name}',
            query => '{query}',
            return_columns => $1,
            num_results => {num_context}
        )",
        );
        let tup_table = c.select(
            &q,
            None,
            Some(vec![(
                PgBuiltInOids::TEXTARRAYOID.oid(),
                columns.into_datum(),
            )]),
        )?;

        for row in tup_table {
            let row_pgrx_js: pgrx::JsonB = row.get_by_name("search_results").unwrap().unwrap();
            let row_js: serde_json::Value = row_pgrx_js.0;

            let record_id = row_js
                .get(&pk)
                .unwrap_or_else(|| error!("`{pk}` not found"));
            let content = row_js
                .get(&content_column)
                .unwrap_or_else(|| error!("`{content_column}` not found"));
            let text_content =
                serde_json::to_string(content).expect("failed to serialize content to string");

            let token_ct = bpe.encode_with_special_tokens(&text_content).len() as i32;
            results.push(ContextualSearch {
                record_id: serde_json::to_string(record_id)
                    .expect("failed to serialize record_id to string"),
                content: text_content,
                token_ct,
            });
        }
        Ok(results)
    });

    let search_results = search_results?;

    // read prompt template
    let handlebars = Handlebars::new();
    let res_prompts: Result<PromptTemplate, spi::Error> = Spi::connect(|c| {
        let q = format!("select * from vectorize.prompts where prompt_type = '{task}'");
        let tup_table = c.select(&q, None, None)?;
        let mut sys_prompt = String::new();
        let mut user_prompt = String::new();
        for row in tup_table {
            sys_prompt = row["sys_prompt"]
                .value::<String>()?
                .expect("sys_prompt is null");
            user_prompt = row["user_prompt"]
                .value::<String>()?
                .expect("user_prompt is null");
        }
        Ok(PromptTemplate {
            sys_prompt,
            user_prompt,
        })
    });
    let p_ok = res_prompts?;

    let sys_prompt_template = p_ok.sys_prompt;
    let user_prompt_template = p_ok.user_prompt;

    let max_context_length = get_context_size(chat_model) as i32;

    // determine how much of the context_limit is consumed by the prompt
    let sys_prompt_token_ct = bpe.encode_with_special_tokens(&sys_prompt_template).len() as i32;
    let user_prompt_token_ct = bpe.encode_with_special_tokens(&user_prompt_template).len() as i32;

    let remaining_tokens = max_context_length - sys_prompt_token_ct - user_prompt_token_ct;

    let prepared_context = prepare_context(&search_results, remaining_tokens, force_trim, &bpe)?;

    let render_vals = serde_json::json!({
        "context_str": prepared_context,
        "query_str": query,
    });
    let user_rendered: String = handlebars.render_template(&user_prompt_template, &render_vals)?;

    let rendered_prompt = RenderedPromt {
        sys_rendered: sys_prompt_template,
        user_rendered,
    };

    // http request to chat completions
    let chat_response = call_chat_completions(rendered_prompt, chat_model, api_key)?;
    Ok(ChatResponse {
        context: search_results,
        chat_response,
    })
}

fn call_chat_completions(
    prompts: RenderedPromt,
    model: &str,
    api_key: Option<&str>,
) -> Result<String> {
    let openai_key = match api_key {
        Some(k) => k.to_string(),
        None => match guc::get_guc(guc::VectorizeGuc::OpenAIKey) {
            Some(k) => k,
            None => {
                error!("failed to get API key from GUC");
            }
        },
    };

    let client = Client::new(openai_key);

    let sys_msg = chat_completion::ChatCompletionMessage {
        role: chat_completion::MessageRole::system,
        content: chat_completion::Content::Text(prompts.sys_rendered),
        name: None,
    };
    let usr_msg = chat_completion::ChatCompletionMessage {
        role: chat_completion::MessageRole::user,
        content: chat_completion::Content::Text(prompts.user_rendered),
        name: None,
    };

    let req = ChatCompletionRequest::new(model.to_string(), vec![sys_msg, usr_msg]);
    let result = client.chat_completion(req)?;
    // currently we only support single query, and not a conversation
    // so we can safely select the first response for now
    let responses = &result.choices[0];
    let chat_response: String = responses
        .message
        .content
        .clone()
        .expect("no response from chat model");
    Ok(chat_response)
}

// Trims the context to fit within the token limit when force_trim = True
// Otherwise returns an error if the context exceeds the token limit
fn prepare_context(
    searches: &[ContextualSearch],
    context_limit: i32,
    force_trim: bool,
    bpe: &CoreBPE,
) -> Result<String> {
    let num_results = searches.len() as i32;
    let total_tokens: i32 = searches.iter().map(|s| s.token_ct).sum();

    // we separate each contextual result with a newline, which adds 1 token
    let total_tokens: i32 = total_tokens + num_results - 1;
    let exceed_limit = total_tokens > context_limit;

    if exceed_limit && !force_trim {
        let err_msg = format!(
            "context exceeds limit: {} > {}",
            total_tokens, context_limit
        );
        return Err(anyhow!(err_msg));
    }

    let combined_string = searches
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<&str>>()
        .join("\n\n");

    let tokens = bpe.split_by_token(&combined_string, true)?;

    // naively trimming the context
    let trimmed_context: String = tokens
        .iter()
        .take(context_limit as usize)
        .cloned()
        .collect::<String>();
    Ok(trimmed_context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_context() {
        let bpe = get_bpe_from_model("gpt-4").unwrap();
        let sentence1 = "This is a test";
        let sentence2 = "This is a much longer test";

        let context = ContextualSearch {
            record_id: "1".to_string(),
            content: sentence1.to_string(),
            token_ct: bpe.encode_with_special_tokens(&sentence1).len() as i32,
        };

        let context2 = ContextualSearch {
            record_id: "2".to_string(),
            content: sentence2.to_string(),
            token_ct: bpe.encode_with_special_tokens(&sentence2).len() as i32,
        };
        let context_str =
            prepare_context(&[context.clone(), context2.clone()], 11, false, &bpe).unwrap();
        assert_eq!("This is a test\n\nThis is a much longer test", context_str);

        // without force_trim, this errors
        let context_str = prepare_context(&[context.clone(), context2.clone()], 1, false, &bpe);
        assert!(context_str.is_err());

        // force trim the result
        let context_str =
            prepare_context(&[context.clone(), context2.clone()], 1, true, &bpe).unwrap();
        assert_eq!("This", context_str);
        let context_str = prepare_context(&[context, context2], 6, true, &bpe).unwrap();
        assert_eq!("This is a test\n\nThis", context_str);
    }
}
