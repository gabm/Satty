use std::f64::consts::PI;

use crate::{
    math::Vec2D,
    sketch_board::{MouseButton, MouseEventMsg},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Default)]
pub struct BrushTool {
    drawable: Option<BrushDrawable>,
    style: Style,
}

#[derive(Debug, Clone)]
pub struct BrushDrawable {
    points: Vec<Vec2D>,
    style: Style,
}

impl Drawable for BrushDrawable {
    fn draw(
        &self,
        cx: &pangocairo::cairo::Context,
        _surface: &pangocairo::cairo::ImageSurface,
    ) -> anyhow::Result<()> {
        let (r, g, b, a) = self.style.color.to_rgba_f64();

        cx.save()?;

        cx.set_line_width(self.style.size.to_line_width());
        cx.set_source_rgba(r, g, b, a);
        cx.set_line_join(pangocairo::cairo::LineJoin::Bevel);

        if self.points.len() == 1 {
            cx.arc(
                self.points[0].x,
                self.points[0].y,
                self.style.size.to_line_width(),
                0.0,
                2.0 * PI,
            );
            cx.fill()?;
        } else if self.points.len() > 1 {
            for p in &self.points {
                cx.line_to(p.x, p.y);
            }
            cx.stroke()?;
        }

        cx.restore()?;

        Ok(())
    }
}

impl Tool for BrushTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event {
            MouseEventMsg::BeginDrag(pos) => {
                self.drawable = Some(BrushDrawable {
                    points: vec![pos],
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventMsg::EndDrag(dir) => {
                if let Some(brush) = &mut self.drawable {
                    // add last point
                    brush.points.push(Vec2D::new(
                        brush.points[0].x + dir.x,
                        brush.points[0].y + dir.y,
                    ));

                    // commit
                    let result = brush.clone_box();
                    self.drawable = None;

                    ToolUpdateResult::Commit(result)
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::UpdateDrag(dir) => {
                if let Some(brush) = &mut self.drawable {
                    // add point
                    brush.points.push(Vec2D::new(
                        brush.points[0].x + dir.x,
                        brush.points[0].y + dir.y,
                    ));

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::Click(pos, button) => {
                if button == MouseButton::Primary {
                    let brush = Box::new(BrushDrawable {
                        points: vec![pos],
                        style: self.style,
                    });
                    ToolUpdateResult::Commit(brush)
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
        }
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.drawable {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }
}
