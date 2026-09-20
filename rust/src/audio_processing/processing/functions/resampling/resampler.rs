use rubato::{FftFixedIn, Resampler};

/// Real-time safe audio resampler wrapping `rubato::FftFixedIn`.
/// Pre-allocates channel scratch buffers to convert native audio (44.1 kHz / 48.0 kHz)
/// to model sample rates (22.05 kHz) with 0 runtime heap allocations.
pub struct AudioResampler {
    resampler: FftFixedIn<f32>,
    in_sample_rate: u32,
    out_sample_rate: u32,
    in_chunk_size: usize,
    input_buffers: Vec<Vec<f32>>,
    output_buffers: Vec<Vec<f32>>,
}

impl AudioResampler {
    /// Creates a new mono audio resampler.
    pub fn new(in_sample_rate: u32, out_sample_rate: u32, in_chunk_size: usize) -> Self {
        let resampler = FftFixedIn::<f32>::new(
            in_sample_rate as usize,
            out_sample_rate as usize,
            in_chunk_size,
            1,
            1,
        )
        .expect("valid resampler configuration");

        let input_buffers = resampler.input_buffer_allocate(true);
        let output_buffers = resampler.output_buffer_allocate(true);

        Self {
            resampler,
            in_sample_rate,
            out_sample_rate,
            in_chunk_size,
            input_buffers,
            output_buffers,
        }
    }

    /// Resamples an input audio slice of length `in_chunk_size``.
    /// Returns a reference to the resampled audio slice in `out_sample_rate`.
    pub fn process_chunk(&mut self, chunk: &[f32]) -> Result<&[f32], Box<dyn std::error::Error>> {
        if chunk.len() < self.in_chunk_size {
            return Err(format!(
                "Input chunk size {} is smaller than expected {}",
                chunk.len(),
                self.in_chunk_size
            )
            .into());
        }

        self.input_buffers[0][..self.in_chunk_size].copy_from_slice(&chunk[..self.in_chunk_size]);

        let (_, out_len) = self
            .resampler
            .process_into_buffer(&self.input_buffers, &mut self.output_buffers, None)
            .map_err(|e| format!("Resampling error: {:?}", e))?;

        Ok(&self.output_buffers[0][..out_len])
    }

    pub fn in_sample_rate(&self) -> u32 {
        self.in_sample_rate
    }

    pub fn out_sample_rate(&self) -> u32 {
        self.out_sample_rate
    }

    pub fn in_chunk_size(&self) -> usize {
        self.in_chunk_size
    }

    pub fn out_chunk_size(&self) -> usize {
        self.resampler.output_frames_max()
    }
}
