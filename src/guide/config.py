from __future__ import annotations

from pydantic_settings import BaseSettings, SettingsConfigDict


class AppConfig(BaseSettings):
    host: str = "0.0.0.0"
    port: int = 8000
    database_url: str = "data/guide.db"
    ollama_base_url: str = "http://localhost:11434/v1"
    default_model: str = "qwen3.5:9b"
    embedding_model: str = "nomic-embed-text"
    ocr_model: str = "glm-ocr"
    vision_model: str = "glm4v"
    cloud_fallback: str | None = None
    cloud_api_key: str | None = None
    max_upload_bytes: int = 50 * 1024 * 1024
    chunk_max_tokens: int = 512
    chunk_overlap_chars: int = 64

    # Hardware acceleration
    # device: "auto" | "cpu" | "cuda" | "cuda:N" | "mps" | "xpu"
    device: str = "auto"
    # num_threads: 0 = auto-detect (os.cpu_count()); >0 = explicit count
    num_threads: int = 0

    model_config = SettingsConfigDict(
        env_prefix="GUIDE__",
        env_nested_delimiter="__",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )
