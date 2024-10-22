pub struct LineSegment {
    pub start: crate::pos::Pos,
    pub end: crate::pos::Pos,
}

impl LineSegment {
    pub fn new(start: crate::pos::Pos, end: crate::pos::Pos) -> Self {
        LineSegment { start, end }
    }

    pub fn line_segments_intersect(&self, other: &LineSegment) -> bool {
        let d1 = direction(&other.start, &other.end, &self.start);
        let d2 = direction(&other.start, &other.end, &self.end);
        let d3 = direction(&self.start, &self.end, &other.start);
        let d4 = direction(&self.start, &self.end, &other.end);

        if d1 != d2 && d3 != d4 {
            return true;
        }

        if d1 == 0 && on_segment(&other.start, &self.start, &other.end) {
            return true;
        }

        if d2 == 0 && on_segment(&other.start, &self.end, &other.end) {
            return true;
        }

        if d3 == 0 && on_segment(&self.start, &other.start, &self.end) {
            return true;
        }

        if d4 == 0 && on_segment(&self.start, &other.end, &self.end) {
            return true;
        }

        false
    }

    pub fn draw_with_outline(
        &self,
        draw: &nannou::Draw,
        color_inner: nannou::color::Rgb<u8>,
        color_outer: nannou::color::Rgb<u8>,
    ) {
        draw.line()
            .start(pt2(self.start.x, self.start.y))
            .end(pt2(self.end.x, self.end.y))
            .weight(3.0)
            .color(color_outer);

        draw.line()
            .start(pt2(self.start.x, self.start.y))
            .end(pt2(self.end.x, self.end.y))
            .weight(1.0)
            .color(color_inner);
    }
}

fn direction(p1: &crate::pos::Pos, p2: &crate::pos::Pos, p3: &crate::pos::Pos) -> i32 {
    let val = (p2.y - p1.y) * (p3.x - p2.x) - (p2.x - p1.x) * (p3.y - p2.y);
    if val == 0.0 {
        0
    } else if val > 0.0 {
        1
    } else {
        -1
    }
}

fn on_segment(p1: &crate::pos::Pos, p2: &crate::pos::Pos, p3: &crate::pos::Pos) -> bool {
    p2.x <= p1.x.max(p3.x) && p2.x >= p1.x.min(p3.x) && p2.y <= p1.y.max(p3.y) && p2.y >= p1.y.min(p3.y)
}
