//! [`faxenoff/code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1) embedder.

#[cfg(feature = "semantic-onnx")]
use crate::semantic_onnx::{Postprocess, SharedOnnxEmbedder};
#[cfg(feature = "semantic-onnx")]
use crate::semantic_onnx_tokenizer::{OnnxTokenizer, resolve_sentencepiece_path};
use rbuilder_error::{Error, Result};
use std::path::{Path, PathBuf};

/// Stable model id stored in semantic indexes built with code-daemon.
pub const CODE_DAEMON_MODEL_ID: &str = "code-daemon:v1";

/// Native embedding width before MRL truncation.
pub const CODE_DAEMON_NATIVE_DIMS: usize = 768;

/// Default max sequence length for code-daemon.
pub const CODE_DAEMON_MAX_SEQ_LEN: usize = 128;

/// Recommended MRL truncation sizes (must be multiples of 8 for binary quant).
pub const CODE_DAEMON_MRL_DIMS: [usize; 3] = [256, 512, 768];

/// Default ONNX filename in the model bundle directory (FP32; INT8 requires newer ORT ops).
pub const CODE_DAEMON_ONNX_FILE: &str = "model.onnx";

/// Default SentencePiece filename in the model bundle directory.
pub const CODE_DAEMON_TOKENIZER_FILE: &str = "sentencepiece.bpe.model";

/// Default model directory under a repository root.
pub fn default_model_dir(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".rbuilder")
        .join("models")
        .join("code-daemon-embed-v1")
}

/// Default ONNX path under a repository root.
pub fn default_model_path(repo_root: &Path) -> PathBuf {
    default_model_dir(repo_root).join(CODE_DAEMON_ONNX_FILE)
}

/// Default SentencePiece path under a repository root.
pub fn default_tokenizer_path(repo_root: &Path) -> PathBuf {
    default_model_dir(repo_root).join(CODE_DAEMON_TOKENIZER_FILE)
}

/// Validate MRL dimensions for code-daemon indexes.
pub fn validate_mrl_dimensions(dimensions: usize) -> Result<()> {
    if dimensions > CODE_DAEMON_NATIVE_DIMS {
        return Err(Error::ConfigError(format!(
            "code-daemon supports at most {CODE_DAEMON_NATIVE_DIMS} dimensions (MRL); got {dimensions}"
        )));
    }
    if dimensions % 8 != 0 {
        return Err(Error::ConfigError(
            "code-daemon dimensions must be a multiple of 8 for binary quantization".into(),
        ));
    }
    Ok(())
}

/// Load code-daemon ONNX embedder with SentencePiece + MRL + L2 normalization.
#[cfg(feature = "semantic-onnx")]
pub fn load_code_daemon_embedder(
    model_path: &Path,
    tokenizer_path: Option<&Path>,
    dimensions: usize,
) -> Result<SharedOnnxEmbedder> {
    validate_mrl_dimensions(dimensions)?;
    let sp_path = resolve_sentencepiece_path(model_path, tokenizer_path)?;
    let tokenizer = OnnxTokenizer::SentencePiece {
        path: sp_path,
        max_seq_len: CODE_DAEMON_MAX_SEQ_LEN,
        bos_id: 2,
        eos_id: 3,
        pad_id: 0,
    };
    SharedOnnxEmbedder::load_with(
        model_path,
        CODE_DAEMON_MODEL_ID,
        dimensions,
        CODE_DAEMON_NATIVE_DIMS,
        tokenizer,
        Postprocess::CodeDaemonMrl,
    )
}
