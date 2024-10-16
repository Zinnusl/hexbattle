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
use wasm_bindgen_futures::JsFuture;

#[cfg(target_family = "wasm")]
use std::sync::RwLock;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

use futures_lite::future::FutureExt;
use std::sync::Arc;
use std::{cell::RefCell, ops::Mul, ops::Sub};

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
    let mut cb = |resolve: js_sys::Function, reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, delay);
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

async fn create_window(app: &App) {
    let device_desc = DeviceDescriptor {
        limits: Limits {
            max_texture_dimension_2d: 8192,
            ..Limits::downlevel_webgl2_defaults()
        },
        ..Default::default()
    };

    app.new_window()
        .size(1024, 1024)
        .device_descriptor(device_desc)
        .title("Hexbattle")
        .view(view)
        // .mouse_pressed(mouse_pressed)
        // .mouse_released(mouse_released)
        .event(event)
        .build_async()
        .await
        .unwrap();
}

fn event(_app: &App, _model: &mut Model, event: WindowEvent) {
    match event {
        WindowEvent::MousePressed(MouseButton::Left) => {
            println!("Mouse pressed");
        }
        WindowEvent::MouseReleased(MouseButton::Left) => {
            println!("Mouse released");
        }
        _ => (),
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
    x: Pos,
    y: Pos,
}

impl LineSegment {
    fn new(x: Pos, y: Pos) -> Self {
        Self { x, y }
    }

    fn shorten(&self, amount: f32) -> Self {
        let x = Pos::new(
            self.x.x + (self.y.x - self.x.x) * amount,
            self.x.y + (self.y.y - self.x.y) * amount,
        );
        let y = Pos::new(
            self.y.x - (self.y.x - self.x.x) * amount,
            self.y.y - (self.y.y - self.x.y) * amount,
        );
        Self { x, y }
    }

    fn line_segments_intersect(&self, other: &LineSegment) -> bool {
        let LineSegment { x: a, y: b } = self.shorten(0.98);
        let LineSegment { x: c, y: d } = other.shorten(0.98);

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
}

#[derive(Clone, Debug)]
struct Anchor {
    pos: Pos,
}

struct Model {
    anchors: Vec<Arc<RefCell<Anchor>>>,
    /// Index into anchors
    dragged_anchor: Option<usize>,
    edges: Vec<(usize, usize)>,
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
        anchors,
        dragged_anchor: None,
        edges: vec![],
    }
}

fn update(app: &App, m: &mut Model, update: Update) {
    // on drag, save the anchor we are dragging in the model
    if app.mouse.buttons.left().is_down() && m.dragged_anchor.is_none() {
        for (i, anchor) in m.anchors.iter().enumerate() {
            let anchor = anchor.borrow();
            if anchor.pos.distance(&Pos::new(app.mouse.x, app.mouse.y)) < 10.0 {
                m.dragged_anchor = Some(i);
            }
        }
    } else if app.mouse.buttons.left().is_up() {
        let dragged_on_anchor_idx = m.anchors.iter().enumerate().find(|(idx, anchor)| {
            anchor
                .borrow()
                .pos
                .distance(&Pos::new(app.mouse.x, app.mouse.y))
                < 10.0
        });

        if let Some(dragged_on_anchor) = dragged_on_anchor_idx {
            let dragged_on_anchor = dragged_on_anchor.0;
            if let Some(dragged_anchor) = m.dragged_anchor {
                if dragged_anchor == dragged_on_anchor {
                    return;
                }

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
        }

        m.dragged_anchor = None;
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
        let anchor = &m.anchors[dragged_anchor];
        draw.line()
            .start(pt2(anchor.borrow().pos.x, anchor.borrow().pos.y))
            .end(pt2(app.mouse.x, app.mouse.y))
            .color(if any_line_intersecting { RED } else { WHEAT });
    }

    // Draw Edges
    for edge in &m.edges {
        let anchor_start = &m.anchors[edge.0];
        let anchor_end = &m.anchors[edge.1];

        let mut points: Vec<Point2> = vec![anchor_start.borrow().pos.into()];

        // Add random points to make the line look like its moving
        let delta_vector = anchor_end.borrow().pos - anchor_start.borrow().pos;
        for i in 1..10 {
            let pos_along_vector = delta_vector * (i as f32 / 10.0);
            points.push(Point2::new(
                anchor_start.borrow().pos.x + pos_along_vector.x + random_range(-2.5, 2.5),
                anchor_start.borrow().pos.y + pos_along_vector.y + random_range(-2.5, 2.5),
            ));
        }

        points.push(anchor_end.borrow().pos.into());

        draw.polyline()
            .weight(4.0)
            .points(points.clone())
            .color(sec_color);
        draw.polyline().weight(2.0).points(points).color(tri_color);
    }

    draw.to_frame(app, &frame).unwrap();
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    println!("Must be run as a web app! Use trunk to build. (cargo install trunk && trunk serve)");
}
