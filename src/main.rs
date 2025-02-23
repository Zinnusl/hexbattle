#![cfg_attr(target_family = "wasm", no_main)]
#![allow(dead_code)]
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
    sync::{Arc, Mutex, atomic::{AtomicU32, Ordering}},
    ops::{Mul, Sub},
};

// Global volume control
static VOLUME: AtomicU32 = AtomicU32::new(0x3F400000); // 0.75 in f32 bits

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
            let mouse_pos = Pos::new(app.mouse.x, app.mouse.y);
            let drag_result = m.interaction.try_start_drag(mouse_pos);
            
            if drag_result.is_some() {
                let current_vol = f32::from_bits(VOLUME.load(Ordering::Relaxed));
                m.freq = Arc::new(Mutex::new(FreqWrapper { value: 100.0, volume: current_vol }));
                m.audio = Some(audio::beep(m.freq.clone()));
                m.last_drag_length = Some(100.0);
            }
        }
        WindowEvent::MouseReleased(MouseButton::Left) => {
            let mouse_pos = Pos::new(app.mouse.x, app.mouse.y);
            m.interaction.try_end_drag(mouse_pos);
            // Signal audio to fade out by setting frequency to 0
            if let Ok(mut freq) = m.freq.lock() {
                freq.value = 0.0;
            }
            
            // Clean up audio immediately - the fade-out will happen in the audio system
            m.audio = None;
            m.last_drag_length = None;
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

/// Represents a 2D position in the graph.
/// 
/// This struct provides basic geometric operations like distance calculation,
/// vector arithmetic (subtraction and scalar multiplication), and conversion
/// to the graphics system's vector type.
#[derive(Clone, Debug, Copy)]
struct Pos {
    /// X coordinate in the 2D space
    x: f32,
    /// Y coordinate in the 2D space
    y: f32,
}

impl Pos {
    /// Creates a new position with the given coordinates.
    ///
    /// # Arguments
    /// * `x` - The x-coordinate
    /// * `y` - The y-coordinate
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculates the Euclidean distance between this position and another.
    ///
    /// # Arguments
    /// * `other` - The other position to calculate distance to
    ///
    /// # Returns
    /// The straight-line distance between the two positions
    fn distance(&self, other: &Pos) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl Into<Vec2> for Pos {
    /// Converts the position into the graphics system's vector type.
    fn into(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}

impl Sub for Pos {
    type Output = Pos;

    /// Implements vector subtraction for positions.
    ///
    /// # Arguments
    /// * `other` - The position to subtract from this one
    ///
    /// # Returns
    /// A new position representing the vector from `other` to `self`
    fn sub(self, other: Self) -> Self::Output {
        Pos::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Pos {
    type Output = Pos;

    /// Implements scalar multiplication for positions.
    ///
    /// # Arguments
    /// * `other` - The scalar value to multiply the position by
    ///
    /// # Returns
    /// A new position with coordinates multiplied by the scalar
    fn mul(self, other: f32) -> Self::Output {
        Pos::new(self.x * other, self.y * other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pos_distance() {
        let p1 = Pos::new(0.0, 0.0);
        let p2 = Pos::new(3.0, 4.0);
        assert_eq!(p1.distance(&p2), 5.0);
    }

    #[test]
    fn test_pos_subtraction() {
        let p1 = Pos::new(5.0, 8.0);
        let p2 = Pos::new(2.0, 3.0);
        let result = p1 - p2;
        assert_eq!(result.x, 3.0);
        assert_eq!(result.y, 5.0);
    }

    #[test]
    fn test_pos_multiplication() {
        let p = Pos::new(2.0, 3.0);
        let result = p * 2.0;
        assert_eq!(result.x, 4.0);
        assert_eq!(result.y, 6.0);
    }

    #[test]
    fn test_line_segment_intersection() {
        // Test intersecting lines
        let l1 = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(10.0, 10.0));
        let l2 = LineSegment::new(Pos::new(0.0, 10.0), Pos::new(10.0, 0.0));
        assert!(l1.line_segments_intersect(&l2));

        // Test parallel lines
        let l3 = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(10.0, 10.0));
        let l4 = LineSegment::new(Pos::new(1.0, 0.0), Pos::new(11.0, 10.0));
        assert!(!l3.line_segments_intersect(&l4));

        // Test non-intersecting lines
        let l5 = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(5.0, 5.0));
        let l6 = LineSegment::new(Pos::new(6.0, 6.0), Pos::new(10.0, 10.0));
        assert!(!l5.line_segments_intersect(&l6));
    }

    #[test]
    fn test_line_segment_shorten() {
        let line = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(10.0, 0.0));
        let shortened = line.shorten_by_fixed_amount(2.0);
        assert_eq!(shortened.start.x, 2.0);
        assert_eq!(shortened.end.x, 8.0);
    }

    #[test]
    fn test_line_segment_zero_length() {
        let line = LineSegment::new(Pos::new(1.0, 1.0), Pos::new(1.0, 1.0));
        let shortened = line.shorten_by_fixed_amount(2.0);
        assert_eq!(shortened.start.x, 1.0);
        assert_eq!(shortened.start.y, 1.0);
        assert_eq!(shortened.end.x, 1.0);
        assert_eq!(shortened.end.y, 1.0);
    }

    #[test]
    fn test_line_segment_parallel() {
        let l1 = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(10.0, 0.0));
        let l2 = LineSegment::new(Pos::new(0.0, 1.0), Pos::new(10.0, 1.0));
        assert!(!l1.line_segments_intersect(&l2));
    }

    #[test]
    fn test_line_segment_shared_endpoint() {
        let l1 = LineSegment::new(Pos::new(0.0, 0.0), Pos::new(10.0, 0.0));
        let l2 = LineSegment::new(Pos::new(10.0, 0.0), Pos::new(10.0, 10.0));
        // Lines that share an endpoint should not be considered intersecting
        assert!(!l1.line_segments_intersect(&l2));
    }

    #[test]
    fn test_anchor_creation() {
        let pos = Pos::new(1.0, 2.0);
        let anchor = Anchor { pos };
        assert_eq!(anchor.pos.x, 1.0);
        assert_eq!(anchor.pos.y, 2.0);
    }
}

/// Represents a line segment between two positions in the graph.
/// 
/// This struct provides functionality for line manipulation and intersection testing,
/// as well as visual rendering with effects.
#[derive(Clone, Debug, Copy)]
struct LineSegment {
    /// Starting point of the line segment
    start: Pos,
    /// Ending point of the line segment
    end: Pos,
}

impl LineSegment {
    /// Creates a new line segment between two positions.
    ///
    /// # Arguments
    /// * `x` - The starting position
    /// * `y` - The ending position
    fn new(x: Pos, y: Pos) -> Self {
        Self { start: x, end: y }
    }

    /// Shortens the line segment from both ends by a proportional factor.
    ///
    /// # Arguments
    /// * `factor` - The proportion of the line length to remove from each end (0.0 to 1.0)
    ///
    /// # Returns
    /// A new line segment with shortened ends
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

    /// Shortens the line segment from both ends by a fixed amount.
    ///
    /// # Arguments
    /// * `amount` - The distance to remove from each end
    ///
    /// # Returns
    /// A new line segment with shortened ends. If the line's length is 0,
    /// returns the original line segment unchanged.
    fn shorten_by_fixed_amount(&self, amount: f32) -> Self {
        let length = self.start.distance(&self.end);
        if length == 0.0 {
            return *self;
        }
        let factor = amount / length;
        self.shorten_with_factor(factor)
    }

    /// Tests if this line segment intersects with another.
    ///
    /// Uses a robust line intersection algorithm that:
    /// 1. Shortens both lines slightly to avoid false positives at endpoints
    /// 2. Calculates intersection point using determinants
    /// 3. Verifies the intersection point lies within both line segments
    ///
    /// # Arguments
    /// * `other` - The other line segment to test intersection with
    ///
    /// # Returns
    /// `true` if the lines intersect, `false` otherwise
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

    /// Draws the line segment with an outline effect and animated distortion.
    ///
    /// Creates a visual effect by:
    /// 1. Drawing a thicker outline behind the main line
    /// 2. Adding random displacement to intermediate points for animation
    /// 3. Using different colors for the line and its outline
    ///
    /// # Arguments
    /// * `draw` - The drawing context
    /// * `color` - The color of the main line
    /// * `outline` - The color of the outline
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

/// Represents a node in the graph that can be connected to other nodes via edges.
///
/// Anchors serve as connection points in the graph and can be:
/// - Created by clicking in empty space
/// - Dragged to create connections
/// - Connected to other anchors via edges
/// - Removed along with their connected edges
#[derive(Clone, Debug)]
struct Anchor {
    /// The position of this anchor in 2D space
    pos: Pos,
}

/// Wraps a frequency value for audio feedback during interactions.
///
/// This wrapper is designed to be shared between threads using Arc<Mutex<_>>
/// and provides thread-safe access to the audio frequency value that changes
/// based on user interactions with the graph.
pub struct FreqWrapper {
    /// The current frequency value for audio feedback
    value: f32,
    /// The current volume level (0.0 to 1.0)
    volume: f32,
}

/// Manages the interactive state of the graph, including anchors (nodes) and edges,
/// as well as drag operations for creating new connections.
#[derive(Debug)]
struct InteractionState {
    /// List of anchor points (nodes) in the graph
    anchors: Vec<Anchor>,
    /// Index of the currently dragged anchor, if any
    dragged_anchor: Option<usize>,
    /// List of edges, each represented as a pair of anchor indices (from, to)
    edges: Vec<(usize, usize)>,
}

impl InteractionState {
    /// Creates a new empty interaction state with no anchors or edges.
    fn new() -> Self {
        Self {
            anchors: Vec::new(),
            dragged_anchor: None,
            edges: Vec::new(),
        }
    }

    /// Creates a new interaction state with pre-existing anchors.
    ///
    /// # Arguments
    /// * `anchors` - Initial set of anchors to populate the state with
    fn with_anchors(anchors: Vec<Anchor>) -> Self {
        Self {
            anchors,
            dragged_anchor: None,
            edges: Vec::new(),
        }
    }

    /// Attempts to start dragging at the given position.
    /// If no anchor exists at the position, creates a new one.
    ///
    /// # Arguments
    /// * `pos` - The position where the drag operation starts
    ///
    /// # Returns
    /// * `Some(index)` if an existing anchor was selected for dragging
    /// * `None` if a new anchor was created
    fn try_start_drag(&mut self, pos: Pos) -> Option<usize> {
        let drag_idx = self.anchors.iter()
            .enumerate()
            .find(|(_idx, anchor)| anchor.pos.distance(&pos) < 10.0)
            .map(|(idx, _)| idx);

        if drag_idx.is_none() {
            self.anchors.push(Anchor { pos });
        }
        
        self.dragged_anchor = drag_idx;
        drag_idx
    }

    /// Attempts to end a drag operation at the given position, potentially creating a new edge.
    ///
    /// # Arguments
    /// * `pos` - The position where the drag operation ends
    ///
    /// # Returns
    /// * `Some((from, to))` if a valid edge was created
    /// * `None` if no edge was created (invalid connection or intersecting with existing edges)
    fn try_end_drag(&mut self, pos: Pos) -> Option<(usize, usize)> {
        let dragged_on_anchor_idx = self.anchors.iter()
            .enumerate()
            .find(|(_, anchor)| anchor.pos.distance(&pos) < 10.0)
            .map(|(idx, _)| idx);

        let new_edge = if let (Some(from), Some(to)) = (self.dragged_anchor, dragged_on_anchor_idx) {
            if from != to && !self.edges.contains(&(from, to)) {
                let new_line = LineSegment::new(
                    self.anchors[from].pos,
                    self.anchors[to].pos,
                );

                let intersecting = self.edges.iter().any(|(a, b)| {
                    let line = LineSegment::new(self.anchors[*a].pos, self.anchors[*b].pos);
                    line.line_segments_intersect(&new_line)
                });

                if !intersecting {
                    self.edges.push((from, to));
                    Some((from, to))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        self.dragged_anchor = None;
        new_edge
    }

    /// Checks if the current drag operation would create an intersecting edge.
    ///
    /// # Arguments
    /// * `current_pos` - The current position of the drag operation
    ///
    /// # Returns
    /// * `true` if the potential edge would intersect with existing edges
    /// * `false` if there's no intersection or no drag operation in progress
    fn is_dragging_intersecting(&self, current_pos: Pos) -> bool {
        if let Some(anchor) = self.dragged_anchor {
            let line = LineSegment::new(self.anchors[anchor].pos, current_pos);
            self.edges.iter().any(|(a, b)| {
                let edge = LineSegment::new(self.anchors[*a].pos, self.anchors[*b].pos);
                edge.line_segments_intersect(&line)
            })
        } else {
            false
        }
    }

    /// Removes all edges from the graph while keeping the anchors.
    fn clear_edges(&mut self) {
        self.edges.clear();
    }

    /// Creates random edges between existing anchors.
    ///
    /// This function:
    /// - Guarantees at least one edge is created if there are 2 or more anchors
    /// - Prevents self-loops (edges from an anchor to itself)
    /// - Avoids duplicate edges
    /// - May create additional random edges between anchors
    ///
    /// Does nothing if there are fewer than 2 anchors.
    fn randomize_edges(&mut self) {
        if self.anchors.len() < 2 {
            return;
        }

        self.edges.clear();
        
        // Ensure at least one edge is created
        let i = random_range(0, self.anchors.len());
        loop {
            let j = random_range(0, self.anchors.len());
            if i != j {
                self.edges.push((i, j));
                break;
            }
        }

        // Add more random edges
        for i in 0..self.anchors.len() {
            let j = random_range(0, self.anchors.len());
            if i != j && !self.edges.contains(&(i, j)) {
                self.edges.push((i, j));
            }
        }
    }

    /// Returns the number of edges in the graph.
    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the number of anchors in the graph.
    fn anchor_count(&self) -> usize {
        self.anchors.len()
    }

    /// Removes an anchor and all its connected edges.
    ///
    /// # Arguments
    /// * `index` - The index of the anchor to remove
    ///
    /// # Returns
    /// * `true` if the anchor was successfully removed
    /// * `false` if the index was invalid
    fn remove_anchor(&mut self, index: usize) -> bool {
        if index >= self.anchors.len() {
            return false;
        }

        // Remove all edges connected to this anchor
        self.edges.retain(|(from, to)| *from != index && *to != index);
        
        // Update edge indices for anchors after the removed one
        for (from, to) in &mut self.edges {
            if *from > index {
                *from -= 1;
            }
            if *to > index {
                *to -= 1;
            }
        }

        self.anchors.remove(index);
        true
    }
}

struct Model {
    interaction: InteractionState,
    audio: Option<audio::Handle>,
    last_drag_length: Option<f32>,
    freq: Arc<Mutex<FreqWrapper>>,
    egui: Option<Egui>,
    wiggle_anchors: bool,
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
        interaction: InteractionState::with_anchors(anchors),
        audio: None,
        last_drag_length: None,
        freq: Arc::new(Mutex::new(FreqWrapper { value: 100.0, volume: f32::from_bits(VOLUME.load(Ordering::Relaxed)) })),
        wiggle_anchors: false,
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;

    fn setup_test_state() -> InteractionState {
        let anchors = vec![
            Anchor { pos: Pos::new(0.0, 0.0) },
            Anchor { pos: Pos::new(100.0, 0.0) },
            Anchor { pos: Pos::new(50.0, 100.0) },
        ];
        InteractionState::with_anchors(anchors)
    }

    #[test]
    fn test_start_drag_on_empty_space() {
        let mut state = InteractionState::new();
        let result = state.try_start_drag(Pos::new(50.0, 50.0));
        assert!(result.is_none());
        assert_eq!(state.anchors.len(), 1);
        assert_eq!(state.anchors[0].pos.x, 50.0);
        assert_eq!(state.anchors[0].pos.y, 50.0);
    }

    #[test]
    fn test_start_drag_on_existing_anchor() {
        let mut state = setup_test_state();
        let result = state.try_start_drag(Pos::new(1.0, 1.0)); // Within 10.0 distance of (0.0, 0.0)
        assert_eq!(result, Some(0));
        assert_eq!(state.dragged_anchor, Some(0));
        assert_eq!(state.anchors.len(), 3); // No new anchor added
    }

    #[test]
    fn test_create_valid_edge() {
        let mut state = setup_test_state();
        state.try_start_drag(Pos::new(1.0, 1.0)); // Start dragging first anchor
        let result = state.try_end_drag(Pos::new(99.0, 1.0)); // End near second anchor
        assert_eq!(result, Some((0, 1)));
        assert_eq!(state.edges.len(), 1);
        assert_eq!(state.edges[0], (0, 1));
    }

    #[test]
    fn test_prevent_self_edge() {
        let mut state = setup_test_state();
        state.try_start_drag(Pos::new(1.0, 1.0)); // Start dragging first anchor
        let result = state.try_end_drag(Pos::new(1.0, 1.0)); // End on same anchor
        assert!(result.is_none());
        assert_eq!(state.edges.len(), 0);
    }

    #[test]
    fn test_prevent_duplicate_edge() {
        let mut state = setup_test_state();
        // Create first edge
        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        // Try to create same edge again
        state.try_start_drag(Pos::new(1.0, 1.0));
        let result = state.try_end_drag(Pos::new(99.0, 1.0));
        assert!(result.is_none());
        assert_eq!(state.edges.len(), 1);
    }

    #[test]
    fn test_prevent_intersecting_edges() {
        let mut state = setup_test_state();
        // Create first edge from (0,0) to (100,0)
        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        // Try to create intersecting edge from (50,100) to (50,-100)
        state.try_start_drag(Pos::new(50.0, 100.0));
        let result = state.try_end_drag(Pos::new(50.0, -100.0));
        assert!(result.is_none());
        assert_eq!(state.edges.len(), 1);
    }

    #[test]
    fn test_clear_edges() {
        let mut state = setup_test_state();
        // Create an edge
        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        assert_eq!(state.edges.len(), 1);
        // Clear edges
        state.clear_edges();
        assert_eq!(state.edges.len(), 0);
    }

    #[test]
    fn test_randomize_edges() {
        let mut state = setup_test_state();
        state.randomize_edges();
        assert!(!state.edges.is_empty());
        // Check that no edge connects an anchor to itself
        for (from, to) in &state.edges {
            assert_ne!(from, to);
        }
    }

    #[test]
    fn test_is_dragging_intersecting() {
        let mut state = setup_test_state();
        // Create horizontal edge from (0,0) to (100,0)
        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        
        // Start new drag from top point
        state.try_start_drag(Pos::new(50.0, 100.0));
        // Check if dragging through the horizontal line intersects
        assert!(state.is_dragging_intersecting(Pos::new(50.0, -100.0)));
        // Check if dragging parallel doesn't intersect
        assert!(!state.is_dragging_intersecting(Pos::new(150.0, 100.0)));
    }

    #[test]
    fn test_edge_and_anchor_count() {
        let mut state = setup_test_state();
        assert_eq!(state.anchor_count(), 3);
        assert_eq!(state.edge_count(), 0);

        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        assert_eq!(state.edge_count(), 1);

        // Add a new anchor by dragging from empty space
        state.try_start_drag(Pos::new(200.0, 200.0));
        assert_eq!(state.anchor_count(), 4);
    }

    #[test]
    fn test_remove_anchor_with_no_edges() {
        let mut state = setup_test_state();
        assert!(state.remove_anchor(1));
        assert_eq!(state.anchor_count(), 2);
        assert_eq!(state.edge_count(), 0);
    }

    #[test]
    fn test_remove_anchor_with_edges() {
        let mut state = setup_test_state();
        // Create two edges: (0,1) and (1,2)
        state.try_start_drag(Pos::new(1.0, 1.0));
        state.try_end_drag(Pos::new(99.0, 1.0));
        state.try_start_drag(Pos::new(99.0, 1.0));
        state.try_end_drag(Pos::new(50.0, 100.0));
        assert_eq!(state.edge_count(), 2);

        // Remove middle anchor (index 1)
        assert!(state.remove_anchor(1));
        assert_eq!(state.anchor_count(), 2);
        assert_eq!(state.edge_count(), 0); // Both edges should be removed
    }

    #[test]
    fn test_remove_anchor_updates_edge_indices() {
        let mut state = setup_test_state();
        // Create edge from last to first anchor: (2,0)
        state.try_start_drag(Pos::new(50.0, 100.0));
        state.try_end_drag(Pos::new(1.0, 1.0));
        assert_eq!(state.edge_count(), 1);

        // Remove middle anchor (index 1)
        assert!(state.remove_anchor(1));
        // Edge indices should be updated: (2,0) -> (1,0)
        assert_eq!(state.edges[0], (1, 0));
    }

    #[test]
    fn test_remove_invalid_anchor() {
        let mut state = setup_test_state();
        assert!(!state.remove_anchor(999)); // Invalid index
        assert_eq!(state.anchor_count(), 3); // No change
    }

    #[test]
    fn test_drag_after_anchor_removal() {
        let mut state = setup_test_state();
        state.remove_anchor(1);
        // Try to drag remaining anchors
        let result = state.try_start_drag(Pos::new(1.0, 1.0));
        assert_eq!(result, Some(0)); // Should still work with updated indices
    }

    #[test]
    fn test_randomize_edges_distribution() {
        let mut state = setup_test_state();
        // Run randomization multiple times to check distribution
        let mut edge_counts = vec![0; 6]; // For 3 anchors, max 6 possible edges
        for _ in 0..100 {
            state.randomize_edges();
            assert!(state.edge_count() > 0); // Should always create some edges
            for (from, to) in &state.edges {
                let edge_index = from * 2 + to;
                edge_counts[edge_index] += 1;
            }
        }
        // Check that all possible edges were used at least once
        assert!(edge_counts.iter().any(|&count| count > 0));
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
        let mouse_pos = Pos::new(app.mouse.x, app.mouse.y);
        let drag_length = m.interaction.dragged_anchor.map(|idx| {
            m.interaction.anchors[idx].pos.distance(&mouse_pos)
        });

        if m.last_drag_length.is_some() && drag_length.is_some() {
            let mut freq = drag_length.unwrap() / 3.0 + 100.0;

            if m.interaction.is_dragging_intersecting(mouse_pos) {
                freq /= random_range(0.25, 0.75);
            }

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
            // Add volume slider

            // Convert between f32 and u32 bits
            let current_vol = f32::from_bits(VOLUME.load(Ordering::Relaxed));
            let mut vol = current_vol;
            
            if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Volume")).changed() {
                VOLUME.store(vol.to_bits(), Ordering::Relaxed);
                if let Ok(mut freq) = m.freq.lock() {
                    freq.volume = vol;
                }
            }
            // Randomize connections button
            ui.label("Randomize connections:");
            if ui.button("Randomize").clicked() {
                m.interaction.randomize_edges();
            }

            ui.label("Clear connections:");
            if ui.button("Clear").clicked() {
                m.interaction.clear_edges();
            }

            ui.label("Wiggle anchors:");
            ui.checkbox(&mut m.wiggle_anchors, "Wiggle");
        });
    }

    if m.wiggle_anchors {
        for anchor in &mut m.interaction.anchors {
            anchor.pos.x += random_range(-1.0, 1.0);
            anchor.pos.y += random_range(-1.0, 1.0);
        }
    }
}

fn view(app: &App, m: &Model, frame: Frame) {
    let main_color = Rgb::new(0x0du8, 0x11u8, 0x17u8);
    let sec_color = Rgb::new(0xf2u8, 0xeeu8, 0xe8u8);
    let tri_color = INDIGO;
    let draw = app.draw();
    draw.background().color(main_color);

    // Draw anchors
    for anchor in &m.interaction.anchors {
        draw.ellipse()
            .x_y(anchor.pos.x, anchor.pos.y)
            .w_h(5.0, 5.0)
            .color(WHEAT);
    }

    // Draw dragged anchor red
    if let Some(dragged_anchor) = m.interaction.dragged_anchor {
        let anchor = &m.interaction.anchors[dragged_anchor];
        draw.ellipse()
            .x_y(anchor.pos.x, anchor.pos.y)
            .w_h(10.0, 10.0)
            .color(RED);
    }

    // Draw uncompleted Line
    if let Some(dragged_anchor) = m.interaction.dragged_anchor {
        let mouse_pos = Pos::new(app.mouse.x, app.mouse.y);
        let line = LineSegment::new(
            m.interaction.anchors[dragged_anchor].pos,
            mouse_pos,
        );
        let is_intersecting = m.interaction.is_dragging_intersecting(mouse_pos);

        let color_inner = if is_intersecting {
            MIDNIGHTBLUE
        } else {
            WHEAT
        };
        let color_outer = if is_intersecting {
            RED
        } else {
            MIDNIGHTBLUE
        };
        line.draw_with_outline(&draw, color_inner, color_outer);
    }

    // Draw Edges
    for edge in &m.interaction.edges {
        let anchor_start = &m.interaction.anchors[edge.0];
        let anchor_end = &m.interaction.anchors[edge.1];

        let line = LineSegment::new(anchor_start.pos, anchor_end.pos);

        let any_line_intersecting = m.interaction.edges.iter().any(|(a, b)| {
            let edge = LineSegment::new(m.interaction.anchors[*a].pos, m.interaction.anchors[*b].pos);
            edge.line_segments_intersect(&line)
        });

        let color_inner = if any_line_intersecting {
            MIDNIGHTBLUE
        } else {
            sec_color
        };
        let color_outer = if any_line_intersecting {
            RED
        } else {
            tri_color
        };
        line.draw_with_outline(&draw, color_inner, color_outer);
    }

    draw.to_frame(app, &frame).unwrap();
    if let Some(egui) = m.egui.as_ref() {
        egui.draw_to_frame(&frame).unwrap();
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    println!("Must be run as a web app! Use trunk to build. (cargo install trunk && trunk serve)");
}
