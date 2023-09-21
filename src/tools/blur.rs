use std::cell::RefCell;

use anyhow::Result;
use pangocairo::cairo::{Context, ImageSurface};
use relm4::gtk::gdk::Key;

use crate::{
    math::{self, Vec2D},
    sketch_board::MouseEventMsg,
    style::{Size, Style},
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Debug)]
pub struct Blur {
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
    editing: bool,
    cached_surface: RefCell<Option<ImageSurface>>,
}

impl Blur {
    fn blur(
        surface: &mut ImageSurface,
        factor: f64,
        pos: Vec2D,
        size: Vec2D,
    ) -> Result<ImageSurface> {
        let (pos, size) = math::rect_ensure_positive_size(pos, size);

        let tmp = ImageSurface::create(
            pangocairo::cairo::Format::ARgb32,
            (size.x / factor) as i32,
            (size.y / factor) as i32,
        )?;

        let tmp_cx = Context::new(tmp.clone())?;
        tmp_cx.scale(1.0 / factor, 1.0 / factor);
        tmp_cx.set_source_surface(surface, -pos.x, -pos.y)?;
        tmp_cx.paint()?;

        let result = ImageSurface::create(
            pangocairo::cairo::Format::ARgb32,
            size.x as i32,
            size.y as i32,
        )?;

        let result_cx = Context::new(result.clone())?;
        result_cx.scale(factor, factor);
        result_cx.set_source_surface(tmp, 0.0, 0.0)?;
        result_cx.paint()?;

        Ok(result)
    }
}

impl Drawable for Blur {
    fn draw(&self, cx: &Context, surface: &ImageSurface) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };
        let (r, g, b) = self.style.color.to_rgb_f64();

        cx.save()?;

        if self.editing {
            // set style
            cx.set_line_width(Size::Medium.to_line_width());
            cx.set_source_rgb(r, g, b);

            // make rect
            cx.rectangle(self.top_left.x, self.top_left.y, size.x, size.y);

            // draw
            cx.stroke()?;
        } else {
            // create new cached image
            if self.cached_surface.borrow().is_none() {
                let mut tmp = surface.clone();
                *self.cached_surface.borrow_mut() = Some(Self::blur(
                    &mut tmp,
                    self.style.size.to_blur_factor(),
                    self.top_left,
                    size,
                )?);
            }

            let (pos, _) = math::rect_ensure_positive_size(self.top_left, size);

            // paint over original
            cx.set_source_surface(self.cached_surface.borrow().as_ref().unwrap(), pos.x, pos.y)?;
            cx.paint()?;
        }

        cx.restore()?;

        Ok(())
    }
}

pub struct BlurTool {
    blur: Option<Blur>,
    style: Style,
}

impl BlurTool {
    pub fn new(style: Style) -> Self {
        Self { blur: None, style }
    }
}

impl Tool for BlurTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event {
            MouseEventMsg::BeginDrag(pos) => {
                // start new
                self.blur = Some(Blur {
                    top_left: pos,
                    size: None,
                    style: self.style,
                    editing: true,
                    cached_surface: RefCell::new(None),
                });

                ToolUpdateResult::Redraw
            }
            MouseEventMsg::EndDrag(dir) => {
                if let Some(a) = &mut self.blur {
                    if dir == Vec2D::zero() {
                        self.blur = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.size = Some(dir);
                        a.editing = false;

                        let result = a.clone_box();
                        self.blur = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::UpdateDrag(dir) => {
                if let Some(a) = &mut self.blur {
                    if dir == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    a.size = Some(dir);

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.blur.is_some() {
            self.blur = None;
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.blur {
            Some(d) => Some(d),
            None => None,
        }
    }
}
