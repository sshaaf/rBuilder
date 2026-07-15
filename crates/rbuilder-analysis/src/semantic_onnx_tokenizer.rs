//! Tokenizers for ONNX semantic embedders.

use rbuilder_error::{Error, Result};
use std::path::{Path, PathBuf};

/// How ONNX inputs are tokenized before inference.
#[derive(Debug, Clone)]
pub enum OnnxTokenizer {
    /// Hash-based token IDs (generic fallback for unknown ONNX models).
    Hash {
        max_seq_len: usize,
        vocab_size: usize,
    },
    /// SentencePiece model (e.g. code-daemon-embed-v1).
    SentencePiece {
        path: PathBuf,
        max_seq_len: usize,
        bos_id: i64,
        eos_id: i64,
        pad_id: i64,
    },
}

impl OnnxTokenizer {
    /// Tokenize one string into `(input_ids, attention_mask)` for batch size 1.
    pub fn encode(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
        match self {
            Self::Hash {
                max_seq_len,
                vocab_size,
            } => Ok(hash_tokenize(text, *max_seq_len, *vocab_size)),
            Self::SentencePiece {
                path,
                max_seq_len,
                bos_id,
                eos_id,
                pad_id,
            } => sentencepiece_encode(text, path, *max_seq_len, *bos_id, *eos_id, *pad_id),
        }
    }
}

/// Resolve SentencePiece path: explicit path, else sibling of the ONNX model.
pub fn resolve_sentencepiece_path(model_path: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if !path.is_file() {
            return Err(Error::ConfigError(format!(
                "tokenizer not found: {}",
                path.display()
            )));
        }
        return Ok(path.to_path_buf());
    }

    let parent = model_path
        .parent()
        .ok_or_else(|| Error::ConfigError("model path has no parent directory".into()))?;

    for name in [
        "sentencepiece.bpe.model",
        "tokenizer.model",
        "spiece.model",
    ] {
        let candidate = parent.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(Error::ConfigError(format!(
        "no SentencePiece tokenizer beside {}; pass --tokenizer PATH",
        model_path.display()
    )))
}

fn hash_tokenize(text: &str, max_seq_len: usize, vocab_size: usize) -> (Vec<i64>, Vec<i64>) {
    let mut ids = Vec::with_capacity(max_seq_len);
    for token in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if token.is_empty() {
            continue;
        }
        if ids.len() >= max_seq_len {
            break;
        }
        ids.push((hash_token(token) % vocab_size as u64) as i64);
    }
    if ids.is_empty() {
        ids.push(0);
    }
    let mask = vec![1i64; ids.len()];
    (ids, mask)
}

fn hash_token(token: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in token.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(feature = "semantic-onnx")]
fn sentencepiece_encode(
    text: &str,
    model_path: &Path,
    max_seq_len: usize,
    bos_id: i64,
    eos_id: i64,
    pad_id: i64,
) -> Result<(Vec<i64>, Vec<i64>)> {
    use sentencepiece_rs::SentencePieceProcessor;

    let sp = SentencePieceProcessor::open(model_path).map_err(|err| {
        Error::ConfigError(format!(
            "load SentencePiece {}: {err}",
            model_path.display()
        ))
    })?;

    let mut ids: Vec<i64> = sp
        .encode_to_ids(text)
        .map_err(|err| Error::ConfigError(format!("tokenize: {err}")))?
        .into_iter()
        .map(|id| id as i64)
        .take(max_seq_len.saturating_sub(2))
        .collect();

    let mut input_ids = vec![bos_id];
    input_ids.append(&mut ids);
    input_ids.push(eos_id);

    let attention_mask: Vec<i64> = input_ids
        .iter()
        .map(|&id| if id == pad_id { 0 } else { 1 })
        .collect();

    Ok((input_ids, attention_mask))
}

#[cfg(not(feature = "semantic-onnx"))]
fn sentencepiece_encode(
    _text: &str,
    _model_path: &Path,
    _max_seq_len: usize,
    _bos_id: i64,
    _eos_id: i64,
    _pad_id: i64,
) -> Result<(Vec<i64>, Vec<i64>)> {
    Err(Error::ConfigError(
        "SentencePiece tokenization requires `--features semantic-onnx`".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_tokenize_never_empty() {
        let (ids, mask) = hash_tokenize("", 16, 1000);
        assert_eq!(ids.len(), 1);
        assert_eq!(mask.len(), ids.len());
    }
}
