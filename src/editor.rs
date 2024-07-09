use button::Button as CustomButton;
use nih_plug::nih_error;
use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::{Arc, Mutex};
use waveform::WaveformBufferOutput;

use crate::BeatrecParams;

pub mod button;
pub mod waveform;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PluginMessage {
    SaveBuffer,
}

#[derive(Lens, Clone)]
pub(crate) struct Data {
    pub(crate) params: Arc<BeatrecParams>,
    pub(crate) buffer_output: Arc<Mutex<WaveformBufferOutput>>,
    pub(crate) recording_progress: Arc<Mutex<f32>>,
    pub(crate) command_sender: crossbeam_channel::Sender<PluginMessage>,
}

pub enum EditorEvent {
    ClickSave,
}

impl Model for Data {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            EditorEvent::ClickSave => self.command_sender.send(PluginMessage::SaveBuffer).unwrap(),
        })
    }
}

const HEIGHT: f32 = 128.0;
const WIDTH: f32 = 512.0;
pub const SPACING: f32 = 12.0;
const BUTTON_HEIGHT: f32 = 24.0;

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (WIDTH as u32, HEIGHT as u32))
}

pub(crate) fn create(editor_data: Data, editor_state: Arc<ViziaState>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::None, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_regular(cx);
        assets::register_noto_sans_bold(cx);

        cx.add_font_mem(include_bytes!("./assets/fonts/bebas.ttf"));

        nih_plug_vizia::vizia_assets::register_roboto_bold(cx);

        if let Err(err) = cx.add_stylesheet(include_style!("src/theme.css")) {
            nih_error!("Failed to load stylesheet: {err:?}")
        }

        editor_data.clone().build(cx);

        VStack::new(cx, |cx| {
            VStack::new(cx, |cx| {
                HStack::new(cx, |cx| {
                    HStack::new(cx, |cx| {
                        Label::new(cx, "Beatrec")
                            .color(Color::rgb(223, 251, 247))
                            .font_size(32.0)
                            .font_family(vec![FamilyOwned::Name(String::from("Bebas Neue"))])
                            .top(Units::Pixels(-8.0))
                            .bottom(Units::Pixels(-12.0));

                        Label::new(cx, "™")
                            .color(Color::rgb(223, 251, 247))
                            .font_size(20.0)
                            .font_family(vec![FamilyOwned::Name(String::from("Bebas Neue"))])
                            .top(Units::Pixels(-8.0))
                            .bottom(Units::Pixels(-12.0))
                            .width(Units::Stretch(1.0));

                        CustomButton::new(
                            cx,
                            |cx| {
                                cx.emit(EditorEvent::ClickSave);
                            },
                            |cx| Label::new(cx, "Export"),
                        );
                    })
                    .width(Units::Stretch(1.0))
                    .height(Units::Auto);
                })
                .height(Units::Auto);

                HStack::new(cx, |cx| {
                    waveform::Waveform::new(cx, Data::buffer_output, Data::recording_progress);
                })
                .height(Units::Pixels(HEIGHT - 3.0 * SPACING - BUTTON_HEIGHT));
            })
            .row_between(Units::Pixels(SPACING))
            .space(Units::Pixels(SPACING));
        })
        .background_color(Color::rgb(14, 16, 20));
    })
}
