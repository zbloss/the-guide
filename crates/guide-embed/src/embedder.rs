use std::sync::{Arc, Mutex};

use ort::{
    ep::ExecutionProviderDispatch,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use tokenizers::Tokenizer;

use guide_core::{GuideError, Result};

/// Prepend to query strings — matches EmbeddingGemma training prompts.
const QUERY_PREFIX: &str = "task: search result | query: ";
/// Prepend to document strings — matches EmbeddingGemma training prompts.
const DOCUMENT_PREFIX: &str = "title: none | text: ";
/// Maximum tokens fed into the model (model supports 2048; we stay conservative).
const MAX_SEQ_LEN: usize = 512;

/// Runs a local ONNX embedding model (e.g. `onnx-community/embeddinggemma-300m-ONNX`)
/// in-process via ONNX Runtime. Downloads model files from HuggingFace Hub on first
/// use and caches them at `~/.cache/huggingface/hub/`.
///
/// The struct is cheaply cloneable (`Arc` internals) and safe to share across threads.
#[derive(Clone)]
pub struct LocalEmbedder {
    /// `Mutex` because `Session::run` takes `&mut self`.
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
}

impl LocalEmbedder {
    /// Download (or reuse cached) model from HuggingFace Hub and initialise the
    /// ONNX Runtime session. Tries GPU execution providers in preference order and
    /// falls back to CPU automatically.
    ///
    /// `model_id` — e.g. `"onnx-community/embeddinggemma-300m-ONNX"`
    ///
    /// **Note:** if the model is gated, set the `HF_TOKEN` environment variable to
    /// your HuggingFace access token before starting the server.
    pub async fn from_pretrained(model_id: &str) -> Result<Self> {
        tracing::info!(model = %model_id, "Downloading/loading local embedding model…");

        let api = hf_hub::api::tokio::Api::new()
            .map_err(|e| GuideError::Internal(e.to_string()))?;
        let repo = api.model(model_id.to_string());

        // Download ONNX model file (cached; only fetches once per machine).
        let model_path = repo.get("onnx/model.onnx").await.map_err(|e| {
            GuideError::Internal(format!(
                "Failed to download ONNX model '{model_id}/onnx/model.onnx': {e}. \
                 If this is a gated model, set the HF_TOKEN environment variable."
            ))
        })?;

        // Some models split weights into a companion data file — download it so
        // ORT can find it in the same cache directory. Silently ignore errors.
        let _ = repo.get("onnx/model.onnx_data").await;

        // Tokenizer lives in the repo root.
        let tokenizer_path = repo.get("tokenizer.json").await.map_err(|e| {
            GuideError::Internal(format!("Failed to download tokenizer for '{model_id}': {e}"))
        })?;

        // Build execution providers before entering the blocking thread so the list
        // is determined at compile time and avoids a clippy vec_init_then_push lint.
        let eps = build_execution_providers();

        // Session and tokenizer construction are CPU-bound — run on a blocking thread.
        let (session, tokenizer) =
            tokio::task::spawn_blocking(move || -> Result<(Session, Tokenizer)> {
                let session = Session::builder()
                    .map_err(|e| GuideError::Internal(e.to_string()))?
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .map_err(|e| GuideError::Internal(e.to_string()))?
                    .with_execution_providers(eps)
                    .map_err(|e| GuideError::Internal(e.to_string()))?
                    .commit_from_file(&model_path)
                    .map_err(|e| GuideError::Internal(e.to_string()))?;

                let tokenizer = Tokenizer::from_file(&tokenizer_path)
                    .map_err(|e| GuideError::Internal(e.to_string()))?;

                Ok((session, tokenizer))
            })
            .await
            .map_err(|e| GuideError::Internal(format!("Embedding init thread panicked: {e}")))??;

        tracing::info!("Local embedding model ready");
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
        })
    }

    /// Embed a search query (prepends the query task prefix).
    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.run_embed(text, QUERY_PREFIX).await
    }

    /// Embed a document being indexed (prepends the document task prefix).
    pub async fn embed_document(&self, text: &str) -> Result<Vec<f32>> {
        self.run_embed(text, DOCUMENT_PREFIX).await
    }

    // ── internals ────────────────────────────────────────────────────────────

    async fn run_embed(&self, text: &str, prefix: &'static str) -> Result<Vec<f32>> {
        let session = Arc::clone(&self.session);
        let tokenizer = Arc::clone(&self.tokenizer);
        let input_text = format!("{prefix}{text}");

        tokio::task::spawn_blocking(move || -> Result<Vec<f32>> {
            // Tokenise (add_special_tokens = true so BOS/EOS are inserted).
            let encoding = tokenizer
                .encode(input_text.as_str(), true)
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            // Truncate to MAX_SEQ_LEN and convert u32 → i64 for ORT.
            let ids: Vec<i64> = encoding
                .get_ids()
                .iter()
                .take(MAX_SEQ_LEN)
                .map(|&x| x as i64)
                .collect();
            let mask: Vec<i64> = encoding
                .get_attention_mask()
                .iter()
                .take(MAX_SEQ_LEN)
                .map(|&x| x as i64)
                .collect();
            let seq_len = ids.len();

            // Build [1, seq_len] tensors for the ONNX session.
            let ids_tensor = Tensor::<i64>::from_array(([1_usize, seq_len], ids))
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            let mask_tensor = Tensor::<i64>::from_array(([1_usize, seq_len], mask))
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            // Run ONNX inference — requires &mut Session.
            let mut sess = session
                .lock()
                .map_err(|e| GuideError::Internal(format!("Session lock poisoned: {e}")))?;
            let outputs = sess
                .run(ort::inputs! {
                    "input_ids"      => ids_tensor,
                    "attention_mask" => mask_tensor,
                })
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            // The model has two outputs: [last_hidden_state, sentence_embedding].
            // We want the second output — the mean-pooled embedding of shape [1, 768].
            let (_shape, emb_data) = outputs[1]
                .try_extract_tensor::<f32>()
                .map_err(|e| GuideError::Internal(e.to_string()))?;

            // L2-normalise the flat embedding slice for cosine similarity.
            let norm: f32 = emb_data.iter().map(|x| x * x).sum::<f32>().sqrt();
            let result: Vec<f32> = if norm > 1e-8 {
                emb_data.iter().map(|x| x / norm).collect()
            } else {
                emb_data.to_vec()
            };

            Ok(result)
        })
        .await
        .map_err(|e| GuideError::Internal(format!("Embedding thread panicked: {e}")))?
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Construct the list of ONNX Runtime execution providers in priority order.
/// GPU providers are only included when the corresponding Cargo feature is enabled.
/// ORT silently falls back to the next provider if the hardware is unavailable.
#[allow(clippy::vec_init_then_push)]
fn build_execution_providers() -> Vec<ExecutionProviderDispatch> {
    let mut eps: Vec<ExecutionProviderDispatch> = Vec::new();

    #[cfg(feature = "cuda")]
    eps.push(ort::ep::CUDA::default().build());

    #[cfg(feature = "directml")]
    eps.push(ort::ep::DirectML::default().build());

    #[cfg(feature = "coreml")]
    eps.push(ort::ep::CoreML::default().build());

    // CPU is always available as the final fallback.
    eps.push(ort::ep::CPU::default().build());
    eps
}
