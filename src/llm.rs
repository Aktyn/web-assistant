mod llm_base;
pub(crate) mod openai;

pub(crate) use llm_base::{ChatMessage, LlmBase as LLM};
pub(crate) use openai::ChatOpenAIConfig;

// pub enum LlmConfig {
//     OpenAI(openai::ChatOpenAIConfig),
// }
