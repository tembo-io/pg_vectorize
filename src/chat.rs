use anyhow::Result;
use handlebars::Handlebars;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use pgrx::prelude::*;
use std::env;

struct PromptTemplate {
    pub sys_prompt: String,
    pub user_prompt: String,
}

struct RenderedPromt {
    pub sys_rendered: String,
    pub user_rendered: String,
}

struct ContextualSearch {
    pub document_name: String,
    pub content: String,
}

pub fn call_chat(agent_name: &str, query: &str, chat_model: &str, task: &str) -> Result<String> {
    // query the relevant vectorize table using the query
    // TODO: refactor so we can call an internal access vector search function
    let search_results: Result<Vec<ContextualSearch>, spi::Error> = Spi::connect(|c| {
        let mut results: Vec<ContextualSearch> = Vec::new();
        let q = format!(
            "
        select search_results from vectorize.search(
            job_name => '{agent_name}',
            query => '{query}',
            return_columns => ARRAY['document_name', 'content'],
            num_results => 2
        )"
        );
        let tup_table = c.select(&q, None, None)?;
        for row in tup_table {
            let row_pgrx_js: pgrx::JsonB = row.get_by_name("search_results").unwrap().unwrap();
            let row_js: serde_json::Value = row_pgrx_js.0;

            results.push(ContextualSearch {
                document_name: row_js
                    .get("document_name")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
                content: row_js.get("content").unwrap().as_str().unwrap().to_string(),
            });
        }
        Ok(results)
    });

    // read prompt template
    let handlebars = Handlebars::new();
    let prompts: Result<PromptTemplate, spi::Error> = Spi::connect(|c| {
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
    let p_ok = prompts?;

    let sys_prompt_template = p_ok.sys_prompt;
    let user_prompt_template = p_ok.user_prompt;
    let combined_content = search_results?
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
    let chat_response = call_chat_completions(rendered_prompt, chat_model)?;
    Ok(chat_response)
}

fn call_chat_completions(prompts: RenderedPromt, model: &str) -> Result<String> {
    let client = Client::new(env::var("OPENAI_API_KEY").unwrap().to_string());

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
