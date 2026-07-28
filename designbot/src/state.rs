use crate::color::Color;
use crate::canvas::TextAlign;

/// Graphics state that can be saved and restored
#[derive(Debug, Clone)]
pub struct GraphicsState {
    pub fill_color: Option<Color>,
    pub stroke_color: Option<Color>,
    pub stroke_width: f64,
    /// Stroke end-cap style (DrawBot `lineCap`): Butt (default), Round, Square.
    pub line_cap: kurbo::Cap,
    /// Stroke join style (DrawBot `lineJoin`): Miter (default), Round, Bevel.
    pub line_join: kurbo::Join,
    /// Miter limit (DrawBot `miterLimit`).
    pub miter_limit: f64,
    /// Dash pattern in on/off lengths (DrawBot `lineDash`); empty = solid.
    pub dash_pattern: Vec<f64>,
    /// Offset into the dash pattern.
    pub dash_offset: f64,
    pub transform: kurbo::Affine,
    pub font_family: Option<String>,
    pub font_size: f64,
    pub text_align: TextAlign,
    /// Active variable-font axis settings as (tag, value) pairs, where `tag` is
    /// the 4-byte OpenType axis tag packed big-endian (e.g. `wght`).
    pub font_variations: Vec<(u32, f32)>,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::black()),
            stroke_color: None,
            stroke_width: 1.0,
            line_cap: kurbo::Cap::Butt,
            line_join: kurbo::Join::Miter,
            miter_limit: 10.0,
            dash_pattern: Vec::new(),
            dash_offset: 0.0,
            transform: kurbo::Affine::IDENTITY,
            font_family: None,
            font_size: 12.0,
            text_align: TextAlign::default(),
            font_variations: Vec::new(),
        }
    }
}

impl GraphicsState {
    #[allow(dead_code)] // convenience constructor; Default is used internally
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the full kurbo stroke from the current state (width, caps, join,
    /// miter limit, dashes) — the single source of truth for stroking.
    pub fn make_stroke(&self) -> kurbo::Stroke {
        let mut stroke = kurbo::Stroke::new(self.stroke_width)
            .with_caps(self.line_cap)
            .with_join(self.line_join)
            .with_miter_limit(self.miter_limit);
        if !self.dash_pattern.is_empty() {
            stroke = stroke.with_dashes(self.dash_offset, self.dash_pattern.iter().copied());
        }
        stroke
    }
}

/// Stack of graphics states for save/restore
#[derive(Debug, Clone)]
pub struct StateStack {
    stack: Vec<GraphicsState>,
}

impl Default for StateStack {
    fn default() -> Self {
        Self {
            stack: vec![GraphicsState::default()],
        }
    }
}

impl StateStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current state
    pub fn current(&self) -> &GraphicsState {
        self.stack.last().unwrap()
    }

    /// Get mutable reference to current state
    pub fn current_mut(&mut self) -> &mut GraphicsState {
        self.stack.last_mut().unwrap()
    }

    /// Save the current state
    pub fn save(&mut self) {
        let current = self.current().clone();
        self.stack.push(current);
    }

    /// Restore the previous state
    pub fn restore(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_stack() {
        let mut stack = StateStack::new();

        // Initial state
        assert_eq!(stack.stack.len(), 1);

        // Modify state
        stack.current_mut().fill_color = Some(Color::rgb(255, 0, 0));

        // Save state
        stack.save();
        assert_eq!(stack.stack.len(), 2);

        // Modify again
        stack.current_mut().fill_color = Some(Color::rgb(0, 255, 0));

        // Restore
        stack.restore();
        assert_eq!(stack.stack.len(), 1);
        assert_eq!(stack.current().fill_color, Some(Color::rgb(255, 0, 0)));
    }
}
