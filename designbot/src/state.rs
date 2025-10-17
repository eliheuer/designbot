use crate::color::Color;

/// Graphics state that can be saved and restored
#[derive(Debug, Clone)]
pub struct GraphicsState {
    pub fill_color: Option<Color>,
    pub stroke_color: Option<Color>,
    pub stroke_width: f64,
    pub transform: kurbo::Affine,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            fill_color: Some(Color::black()),
            stroke_color: None,
            stroke_width: 1.0,
            transform: kurbo::Affine::IDENTITY,
        }
    }
}

impl GraphicsState {
    pub fn new() -> Self {
        Self::default()
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
