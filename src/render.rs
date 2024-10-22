use nannou::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        pub fn view(app: &App, model: &crate::model::Model, frame: Frame) {
            let draw = app.draw();
            draw.background().color(BLACK);

            for (a, b) in &model.edges {
                let line = crate::line_segment::LineSegment::new(model.anchors[*a].pos, model.anchors[*b].pos);
                line.draw_with_outline(&draw, WHITE, GRAY);
            }

            for anchor in &model.anchors {
                draw.ellipse()
                    .x_y(anchor.pos.x, anchor.pos.y)
                    .w_h(10.0, 10.0)
                    .color(WHITE);
            }

            draw.to_frame(app, &frame).unwrap();

            if let Some(egui) = &model.egui {
                egui.draw_to_frame(&frame).unwrap();
            }
        }
    } else {
        pub fn view(app: &App, model: &crate::model::Model, frame: Frame) {
            let draw = app.draw();
            draw.background().color(BLACK);

            for (a, b) in &model.edges {
                let line = crate::line_segment::LineSegment::new(model.anchors[*a].pos, model.anchors[*b].pos);
                line.draw_with_outline(&draw, WHITE, GRAY);
            }

            for anchor in &model.anchors {
                draw.ellipse()
                    .x_y(anchor.pos.x, anchor.pos.y)
                    .w_h(10.0, 10.0)
                    .color(WHITE);
            }

            draw.to_frame(app, &frame).unwrap();

            if let Some(egui) = &model.egui {
                egui.draw_to_frame(&frame).unwrap();
            }
        }
    }
}
