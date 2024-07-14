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
    export_buffer: AudioBuffer,
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

        let (command_sender, command_receiver) = crossbeam_channel::bounded(1);

        Self {
            params: Arc::new(BeatrecParams::default()),
            output_buffer: Vec::new(),
            export_buffer: Vec::new(),
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
                buffer_output: self.waveform_buffer_output.clone(),
                recording_progress: self.recording_progress.clone(),
                command_sender: self.command_sender.clone(),
                is_info_visible: false,
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
        let transport = context.transport();
        let tempo = transport.tempo.unwrap() as f32;
        let is_playing = transport.playing;
        let sample_rate = transport.sample_rate;

        let range_samples;
        match context.transport().loop_range_samples() {
            // If a loop is active in the transport, set range_samples to the amount of samples in that loop
            Some((start, end)) => {
                range_samples = (end - start) as f32;
            }
            // Otherwise, take a single beat (dependent on the transport tempo)
            None => {
                range_samples = sample_rate / (tempo / 60.0);
            }
        }

        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                PluginMessage::SaveBuffer => {
                    let current_buffer = self.export_buffer.clone();

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

                PluginMessage::PlayBuffer => {
                    self.output_buffer.clear();
                    self.output_buffer.append(&mut self.export_buffer.clone());
                }
            }
        }

        for channel_samples in buffer.iter_samples() {
            if is_playing {
                self.recording_buffer
                    .push(channel_samples.into_iter().map(|s| s.to_f32()).collect());

                self.recording_progress
                    .set(self.recording_buffer.len() as f32 / range_samples);

                if self.recording_buffer.len() as f32 >= range_samples {
                    self.export_buffer.clear();
                    self.export_buffer.append(&mut self.recording_buffer);
                    self.recording_buffer.clear();
                }
            } else {
                self.recording_buffer.clear();
            }
        }

        let average_frame_size = (range_samples / 2400.0).round() as usize;

        let chunks = self.export_buffer.chunks_exact(average_frame_size);

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

        if self.output_buffer.len() > 0 {
            let output_slice: Vec<_> = self
                .output_buffer
                .drain(0..buffer.samples().min(self.output_buffer.len() - 1))
                .collect();

            if output_slice.len() > 0 {
                for (i, channel_samples) in buffer.iter_samples().enumerate() {
                    let channel_output_slice = output_slice[i.min(output_slice.len() - 1)].clone();

                    for (j, sample) in channel_samples.into_iter().enumerate() {
                        *sample = channel_output_slice[j];
                    }
                }
            }
        }

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
