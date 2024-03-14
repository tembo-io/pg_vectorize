use serde::Serialize;

pub struct PromptTemplate {
    pub sys_prompt: String,
    pub user_prompt: String,
}

pub struct RenderedPrompt {
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
