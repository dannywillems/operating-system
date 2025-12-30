-- Add LLM context field to users for custom system prompt context
ALTER TABLE users ADD COLUMN llm_context TEXT;
