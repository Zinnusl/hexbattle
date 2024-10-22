use nannou::prelude::*;
use cpal::traits::StreamTrait;

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        pub fn key_pressed(app: &App, model: &mut crate::model::Model, key: Key) {
            match key {
                Key::Space => {
                    if model.audio.is_none() {
                        model.audio = Some(crate::audio::beep(model.freq.clone()));
                    } else {
                        model.audio.take().unwrap().stream.pause().unwrap();
                        model.audio = None;
                    }
                }
                _ => {}
            }
        }

        pub fn event(app: &App, model: &mut crate::model::Model, event: WindowEvent) {
            if let WindowEvent::MousePressed(button) = event {
                if button == MouseButton::Left {
                    let mouse_pos = app.mouse.position();
                    for (i, anchor) in model.anchors.iter().enumerate() {
                        if anchor.pos.distance(&crate::pos::Pos::new(mouse_pos.x, mouse_pos.y)) < 10.0 {
                            model.dragged_anchor = Some(i);
                            break;
                        }
                    }
                }
            } else if let WindowEvent::MouseReleased(button) = event {
                if button == MouseButton::Left {
                    if let Some(dragged_anchor) = model.dragged_anchor {
                        let mouse_pos = app.mouse.position();
                        for (i, anchor) in model.anchors.iter().enumerate() {
                            if i != dragged_anchor
                                && anchor.pos.distance(&crate::pos::Pos::new(mouse_pos.x, mouse_pos.y))
                                    < 10.0
                            {
                                model.edges.push((dragged_anchor, i));
                                break;
                            }
                        }
                    }
                    model.dragged_anchor = None;
                }
            }
        }

        pub fn raw_window_event(
            app: &App,
            model: &mut crate::model::Model,
            event: &nannou::winit::event::WindowEvent,
        ) {
            if let nannou::winit::event::WindowEvent::CursorMoved { position, .. } = event {
                if let Some(dragged_anchor) = model.dragged_anchor {
                    let mouse_pos = app.mouse.position();
                    model.anchors[dragged_anchor].pos = crate::pos::Pos::new(mouse_pos.x, mouse_pos.y);
                }
            }
        }
    } else {
        pub fn key_pressed(app: &App, model: &mut crate::model::Model, key: Key) {
            match key {
                Key::Space => {
                    if model.audio.is_none() {
                        model.audio = Some(crate::audio::beep(model.freq.clone()));
                    } else {
                        model.audio.take().unwrap().stream.pause().unwrap();
                        model.audio = None;
                    }
                }
                _ => {}
            }
        }

        pub fn event(app: &App, model: &mut crate::model::Model, event: WindowEvent) {
            if let WindowEvent::MousePressed(button) = event {
                if button == MouseButton::Left {
                    let mouse_pos = app.mouse.position();
                    for (i, anchor) in model.anchors.iter().enumerate() {
                        if anchor.pos.distance(&crate::pos::Pos::new(mouse_pos.x, mouse_pos.y)) < 10.0 {
                            model.dragged_anchor = Some(i);
                            break;
                        }
                    }
                }
            } else if let WindowEvent::MouseReleased(button) = event {
                if button == MouseButton::Left {
                    if let Some(dragged_anchor) = model.dragged_anchor {
                        let mouse_pos = app.mouse.position();
                        for (i, anchor) in model.anchors.iter().enumerate() {
                            if i != dragged_anchor
                                && anchor.pos.distance(&crate::pos::Pos::new(mouse_pos.x, mouse_pos.y))
                                    < 10.0
                            {
                                model.edges.push((dragged_anchor, i));
                                break;
                            }
                        }
                    }
                    model.dragged_anchor = None;
                }
            }
        }

        pub fn raw_window_event(
            app: &App,
            model: &mut crate::model::Model,
            event: &nannou::winit::event::WindowEvent,
        ) {
            if let nannou::winit::event::WindowEvent::CursorMoved { position, .. } = event {
                if let Some(dragged_anchor) = model.dragged_anchor {
                    let mouse_pos = app.mouse.position();
                    model.anchors[dragged_anchor].pos = crate::pos::Pos::new(mouse_pos.x, mouse_pos.y);
                }
            }
        }
    }
}
