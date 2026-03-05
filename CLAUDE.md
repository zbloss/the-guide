# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**The Guide** is an AI-powered assistant for Dungeon Masters running D&D campaigns. It is currently in the **requirements and planning phase** — no source code exists yet.

## Tech Stack & Architectural Decisions

- **Language:** Rust (core backend)
- **Vector Database:** Qdrant (per-campaign collections)
- **LLM Integration:**
  - Default: Ollama (local inference)
  - All LLM calls use the **OpenAI Rust SDK** pointed at the Ollama OpenAI-compatible endpoint
  - Configurable cloud fallbacks: Anthropic, OpenAI, Gemini
- **PDF Processing:** Vision-based pipeline using GLM-OCR via Ollama; all extraction/reasoning goes through the OpenAI-compatible API (no traditional PDF parsing libraries)
- **Deployment Target:** Single binary or set of binaries; Docker image; packaged executables (.app, .exe)

## Key Constraints

- Use the **OpenAI Rust SDK** for all LLM interactions (including Ollama, GLM-OCR, and vision models) — do not use provider-specific SDKs for inference.
- PDF ingestion must go through the image-to-context pipeline (PDF page → image → GLM-OCR via Ollama), not direct text extraction libraries.
- Qdrant collections should be scoped per campaign to allow cross-campaign switching.

## Commands

> This section will be updated once the Rust project is initialized. Standard commands will follow Cargo conventions:
> - `cargo build` — compile
> - `cargo test` — run all tests
> - `cargo test <test_name>` — run a single test
> - `cargo clippy` — lint
> - `cargo fmt` — format

## Feature Areas (from TASKS.md)

1. Combat Management (initiative, action, state tracking)
2. Campaign Intelligent Assistant (context-aware Q&A, spoiler prevention)
3. Backstory & Character Integration
4. Meaningful Encounter Generation
5. Rulebook & Mechanics Reference (D&D 5e core)
6. PDF Parsing & Campaign Portability
7. Playstyle Personalization
8. World Building & Narrative Tools
9. Session Summaries (tiered: player recaps vs. DM master logs)
