use std::error::Error;
use std::io::{Cursor, Read};
use std::path::Path;
use tract_onnx::prelude::*;

pub type TractPlan = TypedRunnableModel<TypedModel>;

/// Generic, model-agnostic ONNX runtime execution engine using tract-onnx.
pub struct TractEngine {
    plan: Option<TractPlan>,
}

impl TractEngine {
    /// Loads ONNX model weights from a file path.
    pub fn from_file<P: AsRef<Path>>(model_path: P) -> Result<Self, Box<dyn Error>> {
        let plan = tract_onnx::onnx()
            .model_for_path(model_path.as_ref())?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self { plan: Some(plan) })
    }

    /// Convenience alias for loading from a file path.
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, Box<dyn Error>> {
        Self::from_file(model_path)
    }

    /// Loads ONNX model weights from an embedded or in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut cursor = Cursor::new(bytes);
        Self::from_reader(&mut cursor)
    }

    /// Loads ONNX model weights from any reader implementing `Read`.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        let plan = tract_onnx::onnx()
            .model_for_read(&mut reader)?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self { plan: Some(plan) })
    }

    /// Creates a stub / fallback instance with no loaded model plan.
    pub fn new_stub() -> Self {
        Self { plan: None }
    }

    pub fn is_stub(&self) -> bool {
        self.plan.is_none()
    }

    /// Executes inference with a 1D audio slice formatted as [1, N].
    pub fn run_1d(&self, input: &[f32]) -> Result<TVec<TValue>, Box<dyn Error>> {
        if let Some(plan) = &self.plan {
            let arr = tract_ndarray::Array2::from_shape_fn((1, input.len()), |(_, j)| input[j]);
            let tensor: Tensor = arr.into();
            let results = plan.run(tvec!(tensor.into()))?;
            Ok(results)
        } else {
            Ok(tvec![])
        }
    }

    /// Executes inference with a 4D tensor formatted as [B, C, T, F].
    pub fn run_4d(
        &self,
        input: &[f32],
        shape: (usize, usize, usize, usize),
    ) -> Result<TVec<TValue>, Box<dyn Error>> {
        if let Some(plan) = &self.plan {
            let arr = tract_ndarray::Array4::from_shape_fn(
                (shape.0, shape.1, shape.2, shape.3),
                |(b, c, t, f)| {
                    let idx = b * (shape.1 * shape.2 * shape.3)
                        + c * (shape.2 * shape.3)
                        + t * shape.3
                        + f;
                    input.get(idx).copied().unwrap_or(0.0)
                },
            );
            let tensor: Tensor = arr.into();
            let results = plan.run(tvec!(tensor.into()))?;
            Ok(results)
        } else {
            Ok(tvec![])
        }
    }
}

impl Default for TractEngine {
    fn default() -> Self {
        Self::new_stub()
    }
}

unsafe impl Send for TractEngine {}
unsafe impl Sync for TractEngine {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tract_stub_creation() {
        let engine = TractEngine::new_stub();
        assert!(engine.is_stub());

        let audio = [0.0f32; 512];
        let results = engine.run_1d(audio.as_slice()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_tract_default() {
        let engine = TractEngine::default();
        assert!(engine.is_stub());
    }

    #[test]
    fn test_tract_invalid_path() {
        let result = TractEngine::from_file("non_existent_model_file.onnx");
        assert!(result.is_err());
    }
}
