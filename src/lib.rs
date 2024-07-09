use editor::{
    waveform::{WaveformBufferInput, WaveformBufferOutput},
    PluginMessage,
};
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_vizia::ViziaState;
use std::{
    ops::Neg,
    sync::{Arc, Mutex},
};
use triple_buffer::TripleBuffer;

type AudioBuffer = Vec<Vec<f32>>;

pub struct Beatrec {
    params: Arc<BeatrecParams>,
    output_buffer: AudioBuffer,
    waveform_buffer_input: WaveformBufferInput,
    waveform_buffer_output: Arc<Mutex<WaveformBufferOutput>>,
    recording_buffer: AudioBuffer,
    command_sender: crossbeam_channel::Sender<PluginMessage>,
    command_receiver: crossbeam_channel::Receiver<PluginMessage>,
    recording_progress: Arc<Mutex<f32>>,
}

mod editor;

#[derive(Params)]
struct BeatrecParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Beatrec {
    fn default() -> Self {
        let initial_waveform_buffer: Vec<f32> = Vec::new();
        let (waveform_buffer_input, waveform_buffer_output) =
            TripleBuffer::new(&initial_waveform_buffer).split();

        let (command_sender, command_receiver) = crossbeam_channel::bounded(1024);

        Self {
            params: Arc::new(BeatrecParams::default()),
            output_buffer: Vec::new(),
            waveform_buffer_input,
            waveform_buffer_output: Arc::new(Mutex::new(waveform_buffer_output)),
            recording_buffer: Vec::new(),
            command_sender,
            command_receiver,
            recording_progress: Arc::new(Mutex::new(0.0)),
        }
    }
}

impl Default for BeatrecParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
        }
    }
}

impl Plugin for Beatrec {
    const EMAIL: &'static str = "info@example.com";
    const NAME: &'static str = "Beatrec";
    const URL: &'static str = "https://tphbrok.github.io";
    const VENDOR: &'static str = "Thomas Brok";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[],
            aux_output_ports: &[],

            names: PortNames::const_default(),
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            editor::Data {
                params: self.params.clone(),
                buffer_output: self.waveform_buffer_output.clone(),
                recording_progress: self.recording_progress.clone(),
                command_sender: self.command_sender.clone(),
            },
            self.params.editor_state.clone(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let tempo = context.transport().tempo.unwrap() as f32;
        let is_playing = context.transport().playing;
        let sample_rate = context.transport().sample_rate;
        let samples_per_beat;
        let loop_range_samples = context.transport().loop_range_samples();

        match loop_range_samples {
            Some((start, end)) => {
                samples_per_beat = (end - start) as f32;
            }
            None => {
                samples_per_beat = sample_rate / (tempo / 60.0);
            }
        }

        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                PluginMessage::SaveBuffer => {
                    let current_buffer = self.output_buffer.clone();

                    std::thread::spawn(move || {
                        match rfd::FileDialog::new()
                            .set_file_name("Recording.wav")
                            .save_file()
                        {
                            Some(save_file_handle) => {
                                let save_file_path = save_file_handle.as_path();

                                let spec = hound::WavSpec {
                                    channels: 2,
                                    sample_rate: sample_rate as u32,
                                    bits_per_sample: 32,
                                    sample_format: hound::SampleFormat::Float,
                                };
                                let mut writer =
                                    hound::WavWriter::create(save_file_path, spec).unwrap();

                                for sample in current_buffer.into_iter() {
                                    for channel_sample in sample {
                                        writer.write_sample(channel_sample).unwrap();
                                    }
                                }
                            }
                            _ => {}
                        }
                    });
                }
            }
        }

        for channel_samples in buffer.iter_samples() {
            if is_playing {
                self.recording_buffer
                    .push(channel_samples.into_iter().map(|s| s.to_f32()).collect());

                self.recording_progress
                    .set(self.recording_buffer.len() as f32 / samples_per_beat);

                if self.recording_buffer.len() as f32 >= samples_per_beat {
                    self.output_buffer.clear();
                    self.output_buffer.append(&mut self.recording_buffer);
                    self.recording_buffer.clear();
                }
            } else {
                self.recording_buffer.clear();
            }
        }

        let average_frame_size = (samples_per_beat / 2400.0).round() as usize;

        let chunks = self.output_buffer.chunks_exact(average_frame_size);

        let averages = chunks
            .map(|chunk| {
                let mut average = 0.0;

                for channel_samples in chunk {
                    average += channel_samples.iter().sum::<f32>() / channel_samples.len() as f32;
                }

                (average / average_frame_size as f32).clamp(-1.0, 1.0).neg()
            })
            .collect();

        self.waveform_buffer_input.write(averages);

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Beatrec {
    const CLAP_ID: &'static str = "com.tphbrok.beatrec";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("It records your loop or a single beat");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Stereo, ClapFeature::Mono, ClapFeature::Utility];
}

impl Vst3Plugin for Beatrec {
    const VST3_CLASS_ID: [u8; 16] = *b"tphbrokbeatrecaa";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Stereo, Vst3SubCategory::Tools];
}

nih_export_clap!(Beatrec);
nih_export_vst3!(Beatrec);
