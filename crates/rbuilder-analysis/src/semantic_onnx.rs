//! ONNX Runtime embedder (feature `semantic-onnx`).

use crate::semantic_embedder::SemanticEmbedder;
use crate::semantic_onnx_tokenizer::OnnxTokenizer;
use ndarray::{Array2, ArrayD, Axis};
use ort::session::Session;
use ort::value::TensorRef;
use ort::inputs;
use rbuilder_error::{Error, Result};
use std::path::Path;
use std::sync::Mutex;

const DEFAULT_MAX_SEQ_LEN: usize = 128;
const DEFAULT_VOCAB_SIZE: usize = 30_522;

/// Post-inference vector processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Postprocess {
    /// Truncate/pad to target width (generic ONNX).
    Resize,
    /// MRL truncate + L2 normalize (code-daemon).
    CodeDaemonMrl,
}

/// Mutex-backed ONNX embedder safe to share across query threads.
pub struct SharedOnnxEmbedder {
    model_id: String,
    dimensions: usize,
    native_dims: usize,
    input_ids_name: String,
    attention_mask_name: Option<String>,
    tokenizer: OnnxTokenizer,
    postprocess: Postprocess,
    session: Mutex<Session>,
}

impl SharedOnnxEmbedder {
    /// Load a generic ONNX model (hash tokenization unless tokenizer supplied).
    pub fn load(path: &Path, dimensions: usize) -> Result<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("onnx");
        Self::load_with(
            path,
            &format!("onnx:{stem}"),
            dimensions,
            dimensions,
            OnnxTokenizer::Hash {
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                vocab_size: DEFAULT_VOCAB_SIZE,
            },
            Postprocess::Resize,
        )
    }

    /// Load with explicit tokenizer and post-processing.
    pub fn load_with(
        path: &Path,
        model_id: &str,
        dimensions: usize,
        native_dims: usize,
        tokenizer: OnnxTokenizer,
        postprocess: Postprocess,
    ) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::ConfigError(format!(
                "ONNX model not found: {}",
                path.display()
            )));
        }

        let mut builder = Session::builder().map_err(map_ort)?;
        let session = builder.commit_from_file(path).map_err(map_ort)?;

        let input_ids_name = session
            .inputs()
            .first()
            .ok_or_else(|| Error::ConfigError("ONNX model has no inputs".into()))?
            .name()
            .to_string();

        let attention_mask_name = session
            .inputs()
            .iter()
            .map(|input| input.name())
            .find(|name| name.contains("attention"))
            .map(str::to_string);

        Ok(Self {
            model_id: model_id.to_string(),
            dimensions,
            native_dims,
            input_ids_name,
            attention_mask_name,
            tokenizer,
            postprocess,
            session: Mutex::new(session),
        })
    }

    /// Load generic ONNX with optional SentencePiece tokenizer (auto-detect beside model).
    pub fn load_with_optional_tokenizer(
        path: &Path,
        dimensions: usize,
        tokenizer_path: Option<&Path>,
    ) -> Result<Self> {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("onnx");

        let tokenizer = if let Some(explicit) = tokenizer_path {
            let sp_path =
                crate::semantic_onnx_tokenizer::resolve_sentencepiece_path(path, Some(explicit))?;
            OnnxTokenizer::SentencePiece {
                path: sp_path,
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                bos_id: 2,
                eos_id: 3,
                pad_id: 0,
            }
        } else if let Ok(sp_path) =
            crate::semantic_onnx_tokenizer::resolve_sentencepiece_path(path, None)
        {
            OnnxTokenizer::SentencePiece {
                path: sp_path,
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                bos_id: 2,
                eos_id: 3,
                pad_id: 0,
            }
        } else {
            OnnxTokenizer::Hash {
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                vocab_size: DEFAULT_VOCAB_SIZE,
            }
        };

        Self::load_with(
            path,
            &format!("onnx:{stem}"),
            dimensions,
            dimensions,
            tokenizer,
            Postprocess::Resize,
        )
    }
}

impl SemanticEmbedder for SharedOnnxEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let (ids, mask) = self.tokenizer.encode(text)?;
        let seq_len = ids.len();
        let input_ids = Array2::from_shape_vec((1, seq_len), ids).map_err(map_shape)?;
        let attention = Array2::from_shape_vec((1, seq_len), mask).map_err(map_shape)?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::GraphError("ONNX session lock poisoned".into()))?;

        let outputs = if let Some(mask_name) = &self.attention_mask_name {
            session
                .run(inputs![
                    self.input_ids_name.as_str() =>
                        TensorRef::from_array_view(input_ids.view()).map_err(map_ort)?,
                    mask_name.as_str() =>
                        TensorRef::from_array_view(attention.view()).map_err(map_ort)?
                ])
                .map_err(map_ort)?
        } else {
            session
                .run(inputs![
                    self.input_ids_name.as_str() =>
                        TensorRef::from_array_view(input_ids.view()).map_err(map_ort)?
                ])
                .map_err(map_ort)?
        };

        let tensor = outputs[0]
            .try_extract_array::<f32>()
            .map_err(map_ort)?
            .to_owned();

        postprocess_vector(tensor, self.dimensions, self.native_dims, self.postprocess)
    }
}

fn postprocess_vector(
    tensor: ArrayD<f32>,
    dimensions: usize,
    native_dims: usize,
    mode: Postprocess,
) -> Result<Vec<f32>> {
    let mut values = vector_from_output(tensor, native_dims)?;
    match mode {
        Postprocess::Resize => Ok(resize_or_truncate(&values, dimensions)),
        Postprocess::CodeDaemonMrl => {
            if values.len() > dimensions {
                values.truncate(dimensions);
            }
            l2_normalize(&mut values);
            Ok(values)
        }
    }
}

fn vector_from_output(tensor: ArrayD<f32>, native_dims: usize) -> Result<Vec<f32>> {
    match tensor.ndim() {
        1 => Ok(resize_or_truncate(
            tensor
                .as_slice()
                .ok_or_else(|| Error::ConfigError("ONNX output not contiguous".into()))?,
            native_dims,
        )),
        2 => {
            let row = tensor.index_axis(Axis(0), 0);
            Ok(resize_or_truncate(row.as_slice().unwrap_or(&[]), native_dims))
        }
        3 => {
            let batch = tensor.index_axis(Axis(0), 0);
            let hidden = batch.shape()[1];
            let mut pooled = vec![0f32; hidden];
            let seq = batch.shape()[0];
            for seq_idx in 0..seq {
                let row = batch.index_axis(Axis(0), seq_idx);
                for (slot, value) in pooled.iter_mut().zip(row.iter()) {
                    *slot += *value;
                }
            }
            let denom = seq.max(1) as f32;
            for value in &mut pooled {
                *value /= denom;
            }
            Ok(resize_or_truncate(&pooled, native_dims))
        }
        other => Err(Error::ConfigError(format!(
            "unsupported ONNX output rank {other}"
        ))),
    }
}

fn resize_or_truncate(values: &[f32], dimensions: usize) -> Vec<f32> {
    if values.len() == dimensions {
        return values.to_vec();
    }
    if values.len() > dimensions {
        return values[..dimensions].to_vec();
    }
    let mut out = values.to_vec();
    out.resize(dimensions, 0.0);
    out
}

fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for value in values {
            *value /= norm;
        }
    }
}

fn map_ort(err: ort::Error) -> Error {
    Error::ConfigError(format!("ONNX Runtime: {err}"))
}

fn map_shape(err: ndarray::ShapeError) -> Error {
    Error::ConfigError(format!("tensor shape: {err}"))
}
