use nih_plug_vizia::vizia::prelude::*;

use super::BUTTON_HEIGHT;

pub struct Button {
    action: Option<Box<dyn Fn(&mut EventContext)>>,
}

impl Button {
    pub fn new<A, C, V>(cx: &mut Context, action: A, content: C) -> Handle<Self>
    where
        A: 'static + Fn(&mut EventContext),
        C: FnOnce(&mut Context) -> Handle<V>,
        V: 'static + View,
    {
        Self {
            action: Some(Box::new(action)),
        }
        .build(cx, move |cx| {
            (content)(cx).hoverable(false).class("inner");
        })
        .border_radius(Units::Pixels(2.0))
        .border_width(Units::Pixels(1.0))
        .color(Color::rgb(223, 251, 247))
        .cursor(CursorIcon::Hand)
        .default_action_verb(DefaultActionVerb::Click)
        .font_size(14.0)
        .height(Units::Pixels(BUTTON_HEIGHT))
        .navigable(true)
        .role(Role::Button)
    }
}

impl View for Button {
    fn element(&self) -> Option<&'static str> {
        Some("button")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, _meta| match window_event {
            WindowEvent::ActionRequest(action) => match action.action {
                Action::Default => {
                    if let Some(callback) = &self.action {
                        (callback)(cx);
                    }
                }

                _ => {}
            },

            WindowEvent::MouseUp(button) if *button == MouseButton::Left => {
                cx.release();
            }

            WindowEvent::PressDown { mouse } => {
                if *mouse {
                    cx.capture()
                }
                cx.focus();
            }

            _ => {}
        });
    }
}
