mod llm_base;
pub mod openai;

pub use llm_base::{ChatMessage, LlmBase as LLM};
pub use openai::ChatOpenAIConfig;
