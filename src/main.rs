#![cfg_attr(target_family = "wasm", no_main)]
#![allow(dead_code)]
use cpal::traits::StreamTrait;
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
    borrow::BorrowMut,
    sync::{Arc, Mutex},
};
use std::{cell::RefCell, ops::Mul, ops::Sub};

pub mod audio;
pub mod console;
pub mod task;

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
                .key_pressed(key_pressed)
                .raw_event(raw_window_event)
                .title("Hexbattle")
                .view(view)
                // .mouse_pressed(mouse_pressed)
                // .mouse_released(mouse_released)
                .event(event)
                .build_async()
                .await
                .unwrap(),
        )
    };
}

fn key_pressed(_app: &App, _m: &mut Model, key: Key) {
    match key {
        Key::Space => {}
        // Raise the frequency when the up key is pressed.
        Key::Up => {}
        // Lower the frequency when the down key is pressed.
        Key::Down => {}
        _ => (),
    }
}

fn event(app: &App, m: &mut Model, event: WindowEvent) {
    match event {
        WindowEvent::MousePressed(MouseButton::Left) => {
            if m.dragged_anchor.is_none() {
                // for (i, anchor) in m.anchors.iter().enumerate() {
                //     let anchor = anchor.borrow();
                //     if anchor.pos.distance(&Pos::new(app.mouse.x, app.mouse.y)) < 10.0 {
                //         m.dragged_anchor = Some(i);
                //     }
                // }

                m.dragged_anchor = m
                    .anchors
                    .iter()
                    .enumerate()
                    .find(|(_idx, anchor)| {
                        anchor
                            .borrow()
                            .pos
                            .distance(&Pos::new(app.mouse.x, app.mouse.y))
                            < 10.0
                    })
                    .map(|(idx, _)| idx);

                if m.dragged_anchor.is_none() {
                    m.anchors.push(Arc::new(RefCell::new(Anchor {
                        pos: Pos::new(app.mouse.x, app.mouse.y),
                    })));
                } else {
                    m.freq = Arc::new(Mutex::new(FreqWrapper { value: 100.0 }));
                    m.audio = Some(audio::beep(m.freq.clone()));
                    m.last_drag_length = Some(100.0);
                }
            }
        }
        WindowEvent::MouseReleased(MouseButton::Left) => {
            m.audio = None;
            m.last_drag_length = None;

            let dragged_on_anchor_idx = m
                .anchors
                .iter()
                .enumerate()
                .find(|(_, anchor)| {
                    anchor
                        .borrow()
                        .pos
                        .distance(&Pos::new(app.mouse.x, app.mouse.y))
                        < 10.0
                })
                .map(|(idx, _)| idx);

            if dragged_on_anchor_idx.is_some()
                && m.dragged_anchor.is_some()
                && m.dragged_anchor != dragged_on_anchor_idx
                && !m
                    .edges
                    .contains(&(m.dragged_anchor.unwrap(), dragged_on_anchor_idx.unwrap()))
            {
                let dragged_anchor = m.dragged_anchor.unwrap();
                let dragged_on_anchor = dragged_on_anchor_idx.unwrap();

                let new_line = LineSegment::new(
                    m.anchors[dragged_anchor].borrow().pos,
                    m.anchors[dragged_on_anchor].borrow().pos,
                );

                let intersecting = m.edges.iter().any(|(a, b)| {
                    let line =
                        LineSegment::new(m.anchors[*a].borrow().pos, m.anchors[*b].borrow().pos);
                    line.line_segments_intersect(&new_line)
                });

                if !intersecting {
                    m.edges.push((dragged_anchor, dragged_on_anchor));
                }
            }

            m.dragged_anchor = None;
        }
        _ => (),
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle things like keyboard and mouse input.
    if let Some(egui) = model.egui.as_mut() {
        egui.handle_raw_event(event);
    }
}

#[derive(Clone, Debug, Copy)]
struct Pos {
    x: f32,
    y: f32,
}
impl Pos {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    fn distance(&self, other: &Pos) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
impl Into<Vec2> for Pos {
    fn into(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}
impl Sub for Pos {
    type Output = Pos;
    fn sub(self, other: Self) -> Self::Output {
        Pos::new(self.x - other.x, self.y - other.y)
    }
}
impl Mul<f32> for Pos {
    type Output = Pos;
    fn mul(self, other: f32) -> Self::Output {
        Pos::new(self.x * other, self.y * other)
    }
}

#[derive(Clone, Debug, Copy)]
struct LineSegment {
    start: Pos,
    end: Pos,
}

impl LineSegment {
    fn new(x: Pos, y: Pos) -> Self {
        Self { start: x, end: y }
    }

    fn shorten_with_factor(&self, factor: f32) -> Self {
        let x = Pos::new(
            self.start.x + (self.end.x - self.start.x) * factor,
            self.start.y + (self.end.y - self.start.y) * factor,
        );
        let y = Pos::new(
            self.end.x - (self.end.x - self.start.x) * factor,
            self.end.y - (self.end.y - self.start.y) * factor,
        );
        Self { start: x, end: y }
    }

    fn shorten_by_fixed_amount(&self, amount: f32) -> Self {
        let length = self.start.distance(&self.end);
        if length == 0.0 {
            return *self;
        }
        let factor = amount / length;
        self.shorten_with_factor(factor)
    }

    fn line_segments_intersect(&self, other: &LineSegment) -> bool {
        let LineSegment { start: a, end: b } = self.shorten_by_fixed_amount(2.0);
        let LineSegment { start: c, end: d } = other.shorten_by_fixed_amount(2.0);

        let a1 = b.y - a.y;
        let b1 = a.x - b.x;
        let c1 = a1 * a.x + b1 * a.y;

        let a2 = d.y - c.y;
        let b2 = c.x - d.x;
        let c2 = a2 * c.x + b2 * c.y;

        let determinant = a1 * b2 - a2 * b1;

        if determinant == 0.0 {
            return false;
        }

        let x = (b2 * c1 - b1 * c2) / determinant;
        let y = (a1 * c2 - a2 * c1) / determinant;

        let is_on_segment = |a: &Pos, b: &Pos, c: Pos| {
            c.x <= a.x.max(b.x) && c.x >= a.x.min(b.x) && c.y <= a.y.max(b.y) && c.y >= a.y.min(b.y)
        };

        let is_intersecting =
            is_on_segment(&a, &b, Pos::new(x, y)) && is_on_segment(&c, &d, Pos::new(x, y));

        is_intersecting
    }

    fn draw_with_outline(&self, draw: &nannou::draw::Draw, color: Rgb8, outline: Rgb8) {
        let mut points: Vec<Point2> = vec![self.start.into()];

        // Add random points to make the line look like its moving
        let delta_vector = self.end - self.start;
        for i in 1..10 {
            let pos_along_vector = delta_vector * (i as f32 / 10.0);
            points.push(Point2::new(
                self.start.x + pos_along_vector.x + random_range(-2.5, 2.5),
                self.start.y + pos_along_vector.y + random_range(-2.5, 2.5),
            ));
        }

        points.push(self.end.into());

        draw.polyline()
            .weight(4.0)
            .points(points.clone())
            .color(outline);
        draw.polyline().weight(2.0).points(points).color(color);
    }
}

#[derive(Clone, Debug)]
struct Anchor {
    pos: Pos,
}

pub struct FreqWrapper {
    value: f32,
}

struct Model {
    anchors: Vec<Arc<RefCell<Anchor>>>,
    /// Index into anchors
    dragged_anchor: Option<usize>,
    /// Index into anchors
    edges: Vec<(usize, usize)>,
    audio: Option<audio::Handle>,
    last_drag_length: Option<f32>,
    freq: Arc<Mutex<FreqWrapper>>,
    egui: Option<Egui>,
}

fn model() -> Model {
    let rect = Rect::from_w_h(1024.0, 1024.0);
    let anchors_amount = (rect.w() * rect.h() / 1000.0).round() as usize;
    let mut anchors: Vec<Arc<RefCell<Anchor>>> = (0..anchors_amount)
        .map(|_| {
            Arc::new(RefCell::new(Anchor {
                pos: Pos::new(
                    random_range(rect.left(), rect.right()),
                    random_range(rect.top(), rect.bottom()),
                ),
            }))
        })
        .collect();

    // Remove pairs that are too close without messing up indexing
    for i in 0..anchors.len() {
        for j in 0..anchors.len() {
            if i >= anchors.len() || j >= anchors.len() || i == j {
                continue;
            }

            if anchors[i].borrow().pos.distance(&anchors[j].borrow().pos) < 50.0 {
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
                .borrow()
                .pos
                .distance(&Pos::new(app.mouse.x, app.mouse.y))
        });

        if m.last_drag_length.is_some() && drag_length.is_some()
        // && (drag_length.unwrap() - m.last_drag_length.unwrap()).abs() > 0.01
        {
            let mut freq = drag_length.unwrap() / 3.0 + 100.0;

            if let Some(anchor) = m.dragged_anchor {
                let line = LineSegment::new(
                    m.anchors[anchor].borrow().pos,
                    Pos::new(app.mouse.x, app.mouse.y),
                );
                let any_line_intersecting = m.edges.iter().any(|(a, b)| {
                    let edge =
                        LineSegment::new(m.anchors[*a].borrow().pos, m.anchors[*b].borrow().pos);
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
}

fn view(app: &App, m: &Model, frame: Frame) {
    let main_color = Rgb::new(0x0du8, 0x11u8, 0x17u8);
    let sec_color = Rgb::new(0xf2u8, 0xeeu8, 0xe8u8);
    let tri_color = Rgb::new(0x7du8, 0x11u8, 0x17u8);
    let draw = app.draw();
    draw.background().color(main_color);
    // #0d1117;
    // #f2eee8;
    // #7d1117;

    // Draw anchors
    for anchor in &m.anchors {
        draw.ellipse()
            .x_y(anchor.borrow().pos.x, anchor.borrow().pos.y)
            .w_h(5.0, 5.0)
            .color(WHEAT);
    }

    // Draw dragged anchor red
    if let Some(dragged_anchor) = m.dragged_anchor {
        let anchor = &m.anchors[dragged_anchor];
        draw.ellipse()
            .x_y(anchor.borrow().pos.x, anchor.borrow().pos.y)
            .w_h(10.0, 10.0)
            .color(RED);
    }

    // Draw uncompleted Line
    if let Some(dragged_anchor) = m.dragged_anchor {
        let line = LineSegment::new(
            m.anchors[dragged_anchor].borrow().pos,
            Pos::new(app.mouse.x, app.mouse.y),
        );
        let any_line_intersecting = m.edges.iter().any(|(a, b)| {
            let edge = LineSegment::new(m.anchors[*a].borrow().pos, m.anchors[*b].borrow().pos);
            edge.line_segments_intersect(&line)
        });

        let line = LineSegment::new(
            m.anchors[dragged_anchor].borrow().pos,
            Pos::new(app.mouse.x, app.mouse.y),
        );

        let color_inner = if any_line_intersecting {
            MIDNIGHTBLUE
        } else {
            WHEAT
        };
        let color_outer = if any_line_intersecting {
            RED
        } else {
            MIDNIGHTBLUE
        };
        line.draw_with_outline(&draw, color_inner, color_outer);
    }

    // Draw Edges
    for edge in &m.edges {
        let anchor_start = &m.anchors[edge.0];
        let anchor_end = &m.anchors[edge.1];

        let line = LineSegment::new(anchor_start.borrow().pos, anchor_end.borrow().pos);

        line.draw_with_outline(&draw, sec_color, tri_color);
    }

    // Use nannou_egui to draw the UI

    draw.to_frame(app, &frame).unwrap();
    if let Some(egui) = m.egui.as_ref() {
        egui.draw_to_frame(&frame).unwrap();
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    println!("Must be run as a web app! Use trunk to build. (cargo install trunk && trunk serve)");
}
