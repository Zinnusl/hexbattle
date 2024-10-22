#![cfg_attr(target_family = "wasm", no_main)]
#![allow(dead_code)]
#[allow(unused_imports)]
use nannou::prelude::*;
use nannou::{
    app::App,
    wgpu::{DeviceDescriptor, Limits},
};
#[cfg(target_family = "wasm")]
use nannou::{
    app::{self},
    wgpu::Backends,
};
use nannou_egui::{self, egui, Egui};

#[cfg(target_family = "wasm")]
use std::sync::RwLock;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

use std::{
    sync::{Arc, Mutex},
};

pub mod task;
pub mod audio;
pub mod console;
pub mod model;
pub mod pos;
pub mod line_segment;
pub mod input;
pub mod render;

mod server;
mod client;

use crate::model::Model;
use crate::model::Anchor;
use crate::pos::Pos;
use crate::line_segment::LineSegment;

#[cfg(target_family = "wasm")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    thread_local!(static MODEL: RwLock<Option<Model>> = Default::default());
    let model = model();

    MODEL.with(|m| m.write().unwrap().replace(model));

    task::block_on(async {
        app::Builder::new_async(|app| {
            Box::new(async {
                create_window(app).await;
                MODEL.with(|m| m.write().unwrap().take().unwrap())
            })
        })
        .backends(Backends::PRIMARY | Backends::GL)
        .update(update)
        .run_async()
        .await;
    });

    Ok(())
}

#[cfg(not(target_family = "wasm"))]
pub async fn sleep(ms: u32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub async fn sleep(delay: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, delay);
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

static mut WINDOW_ID: Option<WindowId> = None;

async fn create_window(app: &App) {
    let device_desc = DeviceDescriptor {
        limits: Limits {
            max_texture_dimension_2d: 8192,
            ..Limits::downlevel_webgl2_defaults()
        },
        ..Default::default()
    };

    unsafe {
        WINDOW_ID = Some(
            app.new_window()
                .size(1024, 1024)
                .device_descriptor(device_desc)
                .key_pressed(input::key_pressed)
                .raw_event(input::raw_window_event)
                .title("Hexbattle")
                .view(render::view)
                // .mouse_pressed(mouse_pressed)
                // .mouse_released(mouse_released)
                .event(input::event)
                .build_async()
                .await
                .unwrap(),
        )
    };
}

fn model() -> Model {
    let rect = Rect::from_w_h(1024.0, 1024.0);
    let anchors_amount = (rect.w() * rect.h() / 1000.0).round() as usize;
    let mut anchors: Vec<Anchor> = (0..anchors_amount)
        .map(|_| Anchor {
            pos: Pos::new(
                random_range(rect.left(), rect.right()),
                random_range(rect.top(), rect.bottom()),
            ),
        })
        .collect();

    // Remove pairs that are too close without messing up indexing
    for i in 0..anchors.len() {
        for j in 0..anchors.len() {
            if i >= anchors.len() || j >= anchors.len() || i == j {
                continue;
            }

            if anchors[i].pos.distance(&anchors[j].pos) < 50.0 {
                anchors.remove(j);
            }
        }
    }

    Model {
        egui: None,
        anchors,
        dragged_anchor: None,
        edges: vec![],
        audio: None,
        last_drag_length: None,
        freq: Arc::new(Mutex::new(FreqWrapper { value: 100.0 })),
        wiggle_anchors: false,
    }
}

fn update(app: &App, m: &mut Model, update: Update) {
    if m.egui.is_none() {
        let window_id = unsafe { WINDOW_ID.unwrap() };
        let window = app.window(window_id).unwrap();
        m.egui = Some(Egui::from_window(&window));
    }

    // Change the frequency of the sine wave over time.
    if m.audio.is_some() {
        let drag_length = m.dragged_anchor.map(|idx| {
            m.anchors[idx]
                .pos
                .distance(&Pos::new(app.mouse.x, app.mouse.y))
        });

        if m.last_drag_length.is_some() && drag_length.is_some()
        // && (drag_length.unwrap() - m.last_drag_length.unwrap()).abs() > 0.01
        {
            let mut freq = drag_length.unwrap() / 3.0 + 100.0;

            if let Some(anchor) = m.dragged_anchor {
                let line =
                    LineSegment::new(m.anchors[anchor].pos, Pos::new(app.mouse.x, app.mouse.y));
                let any_line_intersecting = m.edges.iter().any(|(a, b)| {
                    let edge = LineSegment::new(m.anchors[*a].pos, m.anchors[*b].pos);
                    edge.line_segments_intersect(&line)
                });

                if any_line_intersecting {
                    freq /= random_range(0.25, 0.75);
                }
            }

            // m.freq.borrow_mut().lock().unwrap().value = freq;
            // Move value closer to target freq, rather than just setting it
            let old_freq = m.freq.borrow_mut().lock().unwrap().value;
            m.freq.borrow_mut().lock().unwrap().value += (freq - old_freq) / 10.0;

            m.last_drag_length = drag_length;
        }
    }

    if let Some(egui) = m.egui.as_mut() {
        egui.set_elapsed_time(update.since_start);
        let ctx = egui.begin_frame();
        egui::Window::new("Settings").show(&ctx, |ui| {
            // Randomize connections button
            ui.label("Randomize connections:");
            if ui.button("Randomize").clicked() {
                m.edges.clear();
                for i in 0..m.anchors.len() {
                    let j = random_range(0, m.anchors.len());
                    m.edges.push((i, j));
                }
            }

            ui.label("Clear connections:");
            if ui.button("Clear").clicked() {
                m.edges.clear();
            }

            ui.label("Wiggle anchors:");
            ui.checkbox(&mut m.wiggle_anchors, "Wiggle");

            // Scale slider
            // ui.label("Scale:");
            // ui.add(egui::Slider::new(&mut settings.scale, 0.0..=1000.0));

            // // Rotation slider
            // ui.label("Rotation:");
            // ui.add(egui::Slider::new(&mut settings.rotation, 0.0..=360.0));

            // // Random color button
            // let clicked = ui.button("Random color").clicked();

            // if clicked {
            //     settings.color = rgb(random(), random(), random());
            // }
        });
    }

    if m.wiggle_anchors {
        for anchor in &mut m.anchors {
            anchor.pos.x += random_range(-1.0, 1.0);
            anchor.pos.y += random_range(-1.0, 1.0);
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        tokio::spawn(async {
            server::start_server("127.0.0.1:8080").await;
        });

        let mut ws_stream = client::connect_to_server("127.0.0.1:8080").await;

        tokio::spawn(async move {
            loop {
                if let Some(message) = client::receive_message(&mut ws_stream).await {
                    println!("Received: {}", message);
                }
            }
        });

        println!("Client connected to server");
    });
}
