//! HUD debug text queue ported from `DebugTextRenderer.java` and `LuaTestHMI.java`.

use mod_api::DrawContext;

#[derive(Clone, Debug, Default)]
pub struct DebugTextRenderer {
    lines: Vec<String>,
}

impl DebugTextRenderer {
    pub fn out(&mut self, line: impl Into<String>) {
        let s = line.into();
        if !self.lines.contains(&s) {
            self.lines.push(s);
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Draws all queued debug lines in the top-left HUD and clears the queue.
    pub fn render_and_clear(&mut self, draw: &mut dyn DrawContext) {
        let mut y = 10.0_f32;
        for line in &self.lines {
            draw.text(line, [10.0, y], 0xFFFF_FFFF, true);
            y += 10.0;
        }
        self.lines.clear();
    }
}
