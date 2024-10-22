cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        pub struct Pos {
            pub x: f32,
            pub y: f32,
        }

        impl Pos {
            pub fn new(x: f32, y: f32) -> Self {
                Pos { x, y }
            }

            pub fn distance(&self, other: &Pos) -> f32 {
                ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
            }
        }

        impl Clone for Pos {
            fn clone(&self) -> Self {
                Pos { x: self.x, y: self.y }
            }
        }
    } else {
        pub struct Pos {
            pub x: f32,
            pub y: f32,
        }

        impl Pos {
            pub fn new(x: f32, y: f32) -> Self {
                Pos { x, y }
            }

            pub fn distance(&self, other: &Pos) -> f32 {
                ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
            }
        }

        impl Clone for Pos {
            fn clone(&self) -> Self {
                Pos { x: self.x, y: self.y }
            }
        }
    }
}
