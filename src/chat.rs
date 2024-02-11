use crate::executor::VectorizeMeta;
use crate::guc;
use crate::util::get_vectorize_meta_spi;

use anyhow::Result;
use handlebars::Handlebars;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use pgrx::prelude::*;
use serde::Serialize;

struct PromptTemplate {
    pub sys_prompt: String,
    pub user_prompt: String,
}

struct RenderedPromt {
    pub sys_rendered: String,
    pub user_rendered: String,
}

#[derive(Debug, Serialize)]
pub struct ContextualSearch {
    pub record_id: String,
    pub content: String,
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
) -> Result<ChatResponse> {
    // get job metadata
    let project_meta: VectorizeMeta = if let Ok(Some(js)) = get_vectorize_meta_spi(agent_name) {
        js
    } else {
        error!("failed to get project metadata");
    };
    use crate::types;
    let job_params = serde_json::from_value::<types::JobParams>(project_meta.params.clone())
        .unwrap_or_else(|e| error!("failed to deserialize job params: {}", e));

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
            num_results => 2
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
            results.push(ContextualSearch {
                record_id: serde_json::to_string(record_id)
                    .expect("failed to serialize record_id to string"),
                content: serde_json::to_string(content)
                    .expect("failed to serialize content to string"),
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
    let combined_content = search_results
        .iter()
        .map(|cs| cs.content.as_str())
        .collect::<Vec<&str>>()
        .join("\n\n");
    let render_vals = serde_json::json!({
        "context_str": combined_content,
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
