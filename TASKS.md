# Project Tasks & Feature Requirements

This document outlines the core features and requirements for **The Guide**, an AI-powered assistant for Dungeon Masters.

## Technical Constraints

Implementation must adhere to the following architectural choices:

- **Language:** Rust (Core Backend & Logic)
- **Vector Storage:** Qdrant (User-defined collections per campaign)
- **LLM Engine:** Ollama (default local), with configurable client support for OpenAI, Anthropic, and Gemini.
  - **SDK:** Use the **OpenAI Rust SDK** for all interactions with the Ollama endpoint.
  - **Vision-OCR/Ingestion:** **GLM-OCR** via Ollama for foundational document extraction, replacing traditional parsing libraries.
- **Portability:** Containerized or easily deployable as a local service. This should compile to a binary, or series of binaries that can eventually be shipped as a docker image and packaged into executables like .app for Mac and .exe for windows.

## Feature Backlog

### 1. Combat Management System

- **Initiative Tracking:** Automated tracking of initiative order for players and NPCs.
- **Action Monitoring:** Keep track of available attacks, spells, and actions for all creatures in an encounter.
- **State Tracking:** Monitor the current state of the battlefield and party member status.

### 2. Campaign Intelligent Assistant

- **Context-Aware Q&A:** Answer questions for both the DM and players based on the current campaign state.
- **Spoiler Prevention:** Filter information provided to players to prevent accidental plot spoilers.
- **Narrative Justification:** Provide the DM with "party-friendly" explanations for mechanical or plot-based restrictions (e.g., why they can't enter a high-level zone yet).

### 3. Backstory & Character Integration

- **Backstory Analysis:** Parse and understand complex character backstories.
- **Plot Weaving:** Automatically suggest ways to weave character motivations, goals, and relationships into the main campaign narrative.

### 4. Meaningful Encounter Generation

- **Contextual Encounters:** Generate "random" encounters that are actually relevant to the plot, subplots, or world-building.
- **Automatic Scaling:** Ensure encounters provide an appropriate challenge for the party's level and composition.
- **Narrative Ties:** Every encounter should feel like it moves the story forward or deepens the world.

### 5. Rulebook & Mechanics Reference

- **Core Rules Engine:** Integrated knowledge of D&D 5e (or other systems) core mechanics.
- **Quick Reference:** Allow players to ask mechanical questions like "What can I do on my turn?" or "How does Grappled work?"

### 6. PDF Parsing & Campaign Portability

- **Document Ingestion:** Ability to parse official or homebrew campaign PDFs.
  - **Image-to-Context Pipeline:** Convert PDF pages to images and use **GLM-OCR** (via Ollama) for end-to-end extraction of text, tables, and layouts.
  - **Consolidated ML Workflow:** All extraction and reasoning must pass through the **OpenAI compatible API** provided by Ollama using the OpenAI Rust SDK.
  - **Visual Reasoning:** Augment GLM-OCR output with general vision models (e.g., Llama 3.2 Vision) for complex maps and non-textual scene descriptions.
- **Lore Extraction:** Automatically extract names, locations, settings, and key plot points to build a campaign-specific knowledge base.
- **Cross-Campaign Support:** Ensure the tool can switch between different campaign settings seamlessly.

### 7. Playstyle Personalization

- **Preference Tracking:** Monitor party preferences for combat vs. social interaction vs. exploration.
- **Dynamic Adaptation:** Suggest encounter alternatives (e.g., a social challenge instead of a combat encounter) based on tracked preferences.

### 8. World Building & Narrative Tools

- **Cinematic Narratives:** Generate high-quality opening sequences or milestone narratives similar to professional RPGs (e.g., Elden Ring style intros).
- **Perspective Management:** Maintain a clear distinction between what is known by the DM vs. what has been revealed to the players.

### 9. Session Summaries & Player Recaps

- **Gameplay Tracking:** Monitor and log key events, decisions, and discoveries during a play session.
- **Tiered Summary Generation:**
  - **Player Recaps:** Generate summaries that highlight important information covered, items found, and NPCs met, while strictly filtering out any campaign spoilers or unrevealed plot points.
  - **DM Master Logs:** Generate comprehensive summaries including "behind the curtain" details, plot progress, and how session events impact future campaign milestones.
- **Narrative Consistency:** Ensure that recaps use the established tone of the campaign and character-specific hooks where applicable.
