use crate::model::FreqWrapper;

pub struct Model {
    pub egui: Option<nannou_egui::Egui>,
    pub anchors: Vec<Anchor>,
    pub dragged_anchor: Option<usize>,
    pub edges: Vec<(usize, usize)>,
    pub audio: Option<crate::audio::Handle>,
    pub last_drag_length: Option<f32>,
    pub freq: std::sync::Arc<std::sync::Mutex<FreqWrapper>>,
    pub wiggle_anchors: bool,
}

pub struct Anchor {
    pub pos: crate::pos::Pos,
}

pub struct FreqWrapper {
    pub value: f32,
}
