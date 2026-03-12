# Partner DM Narrator — Agent Memory

## Project: The Guide (Rust/axum + React/TypeScript)

### Architecture Quick Reference
- Backend: Rust stable, axum 0.8, sqlx 0.8 SQLite, `async-openai` -> Ollama
- Frontend: React + TypeScript, bun package manager, `guide-frontend/`
- LLM tasks: `LlmTask` enum in `crates/guide-llm/src/client.rs`
- Prompts: `crates/guide-llm/src/prompts.rs`
- Core models: `crates/guide-core/src/models/` (session.rs, character.rs, shared.rs)
- API routes: `crates/guide-api/src/routes/` — register new routes in `router()` AND `openapi.rs`
- Frontend API: `guide-frontend/src/api/` — types.ts, sessions.ts, characters.ts

### Linter Behavior (Important)
The project has an aggressive linter/formatter that reverts partial edits to Rust files.
When modifying `.rs` files in `guide-api/src/routes/`:
- Use Bash `cat >` or Write tool with the COMPLETE file content, not incremental edits
- Always re-read the file before each Edit to get the current state
- The linter may also expand files with additional WIP code — read before assuming content

### Adding New LLM Features Pattern
1. Add `LlmTask::MyTask` to `crates/guide-llm/src/client.rs`
2. Add `my_system_prompt()` to `crates/guide-llm/src/prompts.rs`
3. Add response model (e.g. `MyResponse { ... }`) to the relevant `guide-core/src/models/` file
4. Add handler to the route file (`sessions.rs` or `characters.rs`)
5. Register route in `router()` function of that file
6. Register path in `openapi.rs` paths list
7. Register schema in `openapi.rs` components list
8. Add TypeScript types to `guide-frontend/src/api/types.ts`
9. Add API function to the relevant `guide-frontend/src/api/` file
10. Add UI to the relevant page component

### Key JSON Stripping Pattern (LLM responses)
```rust
let json_str = raw
    .trim_start_matches("```json")
    .trim_start_matches("```")
    .trim_end_matches("```")
    .trim();
```

### File Download from axum
```rust
use axum::{body::Body, http::{header, StatusCode}, response::Response};
Response::builder()
    .status(StatusCode::OK)
    .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
    .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
    .body(Body::from(content))
    .unwrap()
```
Handler return type must be `Result<Response, AppError>` not `Result<impl IntoResponse, AppError>`.

### Pre-existing WIP in Working Tree (2026-03-11)
- `campaigns.rs` had stubs for `search_campaign` and `generate_atmosphere` (now implemented)
- `character.rs` had `SpellSlot`/`spell_slots` added to `CreateCharacterRequest`
- `characters.rs` had `spend_slot`/`restore_slot` WIP handlers
- These were already in the working tree — not introduced by FEAT-007/009/010 work
