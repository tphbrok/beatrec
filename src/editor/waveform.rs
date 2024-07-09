use nih_plug_vizia::vizia::{
    prelude::*,
    vg::{Color, Paint, Path},
};
use std::sync::{Arc, Mutex};

use super::SPACING;

pub type WaveformBuffer = Vec<f32>;
pub type WaveformBufferInput = triple_buffer::Input<WaveformBuffer>;
pub type WaveformBufferOutput = triple_buffer::Output<WaveformBuffer>;

pub struct Waveform {
    buffer_output: Arc<Mutex<WaveformBufferOutput>>,
    recording_progress: Arc<Mutex<f32>>,
}

impl Waveform {
    pub fn new<LBufferOutput, LRecordingProgress>(
        cx: &mut Context,
        buffer_output: LBufferOutput,
        recording_progress: LRecordingProgress,
    ) -> Handle<Self>
    where
        LBufferOutput: Lens<Target = Arc<Mutex<WaveformBufferOutput>>>,
        LRecordingProgress: Lens<Target = Arc<Mutex<f32>>>,
    {
        Self {
            buffer_output: buffer_output.get(cx),
            recording_progress: recording_progress.get(cx),
        }
        .build(cx, |_cx| ())
    }
}

impl View for Waveform {
    fn element(&self) -> Option<&'static str> {
        Some("waveform")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        let paint = Paint::color(Color::rgb(223, 251, 247)).with_line_width(2.0);

        let recording_progress = self.recording_progress.lock().unwrap().clone();
        let mut progress_path = Path::new();

        progress_path.move_to(bounds.x, bounds.y);
        progress_path.line_to(bounds.x + bounds.w * recording_progress, bounds.y);

        canvas.stroke_path(
            &progress_path,
            &Paint::color(Color::rgba(223, 251, 247, 255)).with_line_width(2.0),
        );

        let base_y = bounds.h / 2.0 + bounds.y + SPACING;
        let mut buffer = self.buffer_output.lock().unwrap();
        let buffer = buffer.read();
        let buffer_len = buffer.len() as f32;
        let mut path = Path::new();

        for (index, sample) in buffer.into_iter().enumerate() {
            let x = bounds.x + index as f32 / buffer_len * bounds.w;

            let new_y = base_y + (sample * (bounds.h - 2.0 * SPACING) / 2.0);

            path.line_to(x, new_y);
            path.move_to(x, new_y);
        }

        canvas.stroke_path(&path, &paint);
    }
}
