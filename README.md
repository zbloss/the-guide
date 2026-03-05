# The Guide

**The Guide** is a specialized assistant designed to help Dungeon Masters and Game Managers run Dungeons & Dragons (DnD) campaigns with greater efficiency, depth, and narrative consistency.

## Overview

Whether you're a first-time DM or an experienced pro, running a successful campaign presents unique challenges. Balancing party engagement, complex rules, and long-term plot threads is a difficult feat.

Think of **The Guide** as your "Partner DM." It is built to assist with combat tracking, narrative consistency, character integration, and world-building, ensuring that your party stays immersed in the story without the mechanical overhead slowing you down.

## Vision

The goal of this project is to create an intelligent companion that:

- **Protects the Plot:** Ensures DM decisions don't accidentally break future plot points.
- **Deepens Immersion:** Weaves player backstories directly into the campaign fabric.
- **Simplifies Mechanics:** Handles the "busy work" of initiative and rule lookups.
- **Personalizes the Experience:** Adapts the campaign's tone and encounter types to match your party's specific playstyle.

## Tech Stack

The Guide is built with a focus on performance, privacy, and extensibility:

- **Core Backend:** Rust (for high performance and safety)
- **Vector Database:** [Qdrant](https://qdrant.tech/) (for efficient campaign knowledge retrieval)
- **LLM Integration:**
  - **Local-First:** [Ollama](https://ollama.com/) for all inference (using the OpenAI Rust SDK for compatibility).
  - **Cloud Providers:** Configurable support for Anthropic, OpenAI, and Gemini.
- **PDF Processing:**
  - **Vision-based Ingestion:** [GLM-OCR](https://huggingface.co/zai-org/GLM-OCR) via Ollama for end-to-side extraction of text, tables, and layouts directly from document images.
  - **Visual Reasoning:** General Ollama Vision models (e.g., Llama 3.2 Vision) for supplementary map and scene description.
- **Frontend:** Potential future development using JavaScript/TypeScript.

## Feature Roadmap

Detailed technical requirements and the feature backlog are maintained in the [TASKS.md](TASKS.md) file. This includes plans for:

1. **Combat Management System**
2. **Context-Aware Campaign Intelligence**
3. **Automated Character Background Integration**
4. **Dynamic Plot-Relevant Encounters**
5. **PDF Campaign Parsing & Lore Extraction**
6. **Intelligent Session Summaries (Tiered for DM/Players)**

## Project Status

This project is currently in the **requirements and planning phase**. The files in this repository serve as the blueprint for a fleet of AI agents that will soon begin the implementation process.

---

_Developed for DMs who want to spend less time checking tables and more time telling stories._
