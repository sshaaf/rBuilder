//! Pluggable semantic embedders (sign-hash default; ONNX optional).

use rbuilder_error::{Error, Result};
use std::path::{Path, PathBuf};

use crate::semantic_search::{quantize_binary, sign_hash_embed, SIGN_HASH_MODEL_ID};

/// Embed text into a float vector before binary quantization.
pub trait SemanticEmbedder: Send + Sync {
    /// Stable model identifier stored in [`crate::semantic_search::SemanticIndex`].
    fn model_id(&self) -> &str;
    /// Output float dimensions (must match index configuration).
    fn dimensions(&self) -> usize;
    /// Embed one document/query string.
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// Embed and sign-quantize to bit-packed bytes.
    fn embed_binary(&self, text: &str) -> Result<Vec<u8>> {
        Ok(quantize_binary(&self.embed(text)?))
    }
}

/// Deterministic hash embedder (always available).
#[derive(Debug, Clone)]
pub struct SignHashEmbedder {
    dimensions: usize,
}

impl SignHashEmbedder {
    /// Create a sign-hash embedder with the given output width.
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl SemanticEmbedder for SignHashEmbedder {
    fn model_id(&self) -> &str {
        SIGN_HASH_MODEL_ID
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(sign_hash_embed(text, self.dimensions))
    }
}

/// How the caller wants to embed during `semantic index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedderChoice {
    /// Built-in sign-hash (`sign-hash-v1`).
    SignHash,
    /// Generic ONNX `--model` (hash tokenization, or SentencePiece with `--tokenizer`).
    Onnx {
        model: PathBuf,
        tokenizer: Option<PathBuf>,
    },
    /// [`faxenoff/code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1).
    CodeDaemon {
        model: PathBuf,
        tokenizer: Option<PathBuf>,
    },
}

/// Options when reloading an ONNX-backed index at query time.
#[derive(Debug, Clone, Default)]
pub struct OnnxReloadOptions {
    pub model_path: Option<PathBuf>,
    pub tokenizer_path: Option<PathBuf>,
}

/// Resolve an embedder for indexing or querying.
pub fn resolve_embedder(choice: &EmbedderChoice, dimensions: usize) -> Result<Box<dyn SemanticEmbedder>> {
    match choice {
        EmbedderChoice::SignHash => Ok(Box::new(SignHashEmbedder::new(dimensions))),
        EmbedderChoice::Onnx { model, tokenizer } => {
            onnx_embedder(model, dimensions, tokenizer.as_deref())
        }
        EmbedderChoice::CodeDaemon { model, tokenizer } => code_daemon_embedder(
            model,
            dimensions,
            tokenizer.as_deref(),
        ),
    }
}

/// Resolve embedder from a persisted index (and optional ONNX path overrides).
pub fn embedder_for_index(
    index: &crate::semantic_search::SemanticIndex,
    reload: &OnnxReloadOptions,
) -> Result<Box<dyn SemanticEmbedder>> {
    if index.model_id == SIGN_HASH_MODEL_ID {
        return Ok(Box::new(SignHashEmbedder::new(index.dimensions)));
    }
    if index.model_id == crate::semantic_code_daemon::CODE_DAEMON_MODEL_ID {
        let model = reload
            .model_path
            .clone()
            .or_else(|| index.model_path.clone().map(PathBuf::from))
            .ok_or_else(|| {
                Error::ConfigError(
                    "code-daemon index requires --model path (or rebuild index with model_path stored)"
                        .into(),
                )
            })?;
        let tokenizer = reload
            .tokenizer_path
            .clone()
            .or_else(|| index.tokenizer_path.clone().map(PathBuf::from));
        return code_daemon_embedder(
            &model,
            index.dimensions,
            tokenizer.as_deref(),
        );
    }
    if index.model_id.starts_with("onnx:") {
        let model = reload
            .model_path
            .clone()
            .or_else(|| index.model_path.clone().map(PathBuf::from))
            .ok_or_else(|| {
                Error::ConfigError(
                    "ONNX index requires --model path (or rebuild index with model_path stored)"
                        .into(),
                )
            })?;
        let tokenizer = reload
            .tokenizer_path
            .clone()
            .or_else(|| index.tokenizer_path.clone().map(PathBuf::from));
        return onnx_embedder(
            &model,
            index.dimensions,
            tokenizer.as_deref(),
        );
    }
    Err(Error::ConfigError(format!(
        "unknown semantic model_id: {}",
        index.model_id
    )))
}

fn onnx_embedder(
    path: &Path,
    dimensions: usize,
    tokenizer: Option<&Path>,
) -> Result<Box<dyn SemanticEmbedder>> {
    #[cfg(feature = "semantic-onnx")]
    {
        Ok(Box::new(
            super::semantic_onnx::SharedOnnxEmbedder::load_with_optional_tokenizer(
                path, dimensions, tokenizer,
            )?,
        ))
    }
    #[cfg(not(feature = "semantic-onnx"))]
    {
        let _ = (path, dimensions, tokenizer);
        Err(Error::ConfigError(
            "ONNX embedder requires building with `--features semantic-onnx`".into(),
        ))
    }
}

fn code_daemon_embedder(
    path: &Path,
    dimensions: usize,
    tokenizer: Option<&Path>,
) -> Result<Box<dyn SemanticEmbedder>> {
    #[cfg(feature = "semantic-onnx")]
    {
        Ok(Box::new(super::semantic_code_daemon::load_code_daemon_embedder(
            path, tokenizer, dimensions,
        )?))
    }
    #[cfg(not(feature = "semantic-onnx"))]
    {
        let _ = (path, dimensions, tokenizer);
        Err(Error::ConfigError(
            "code-daemon embedder requires building with `--features semantic-onnx`".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_hash_embedder_round_trip() {
        let embedder = SignHashEmbedder::new(64);
        let floats = embedder.embed("authenticate token").unwrap();
        assert_eq!(floats.len(), 64);
        let bits = embedder.embed_binary("authenticate token").unwrap();
        assert_eq!(bits.len(), 8);
    }
}
