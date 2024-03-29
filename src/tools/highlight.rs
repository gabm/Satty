use std::cell::RefCell;

use anyhow::Result;
use femtovg::{imgref::Img, rgb::RGBA, ImageFlags, ImageId, Paint, Path};

use relm4::gtk::gdk::Key;

use crate::{
    math::{self, Vec2D},
    sketch_board::{MouseEventMsg, MouseEventType},
    style::{Size, Style},
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Debug)]
pub struct Highlight {
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
    editing: bool,
    cached_image: RefCell<Option<ImageId>>,
}

impl Highlight {
    fn highlight(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        pos: Vec2D,
        size: Vec2D,
    ) -> Result<ImageId> {
        let strength = match self.style.size {
            Size::Large => 0.5,
            Size::Medium => 0.4,
            Size::Small => 0.2,
        };
        let img = canvas.screenshot()?;
        let scale = canvas.transform().average_scale();
        let scaled_width = canvas.width() as f32 / scale;
        let dpi = img.width() as f32 / scaled_width;

        let scaled_x = (pos.x * dpi).round();
        let scaled_y = (pos.y * dpi).round();
        let scaled_width = (size.x * dpi).round();
        let scaled_height = (size.y * dpi).round();

        // error when any size dim is 0 since img.sub_image panics
        if scaled_width == 0. || scaled_height == 0. {
            return Err(anyhow::anyhow!("width or height is 0"));
        }
        let sub = img.sub_image(
            scaled_x as usize,
            scaled_y as usize,
            scaled_width as usize,
            scaled_height as usize,
        );
        let new_buf = sub
            .pixels()
            .map(|pixel| {
                // Alternate style
                // RGBA::new(
                //     pixel.r.min(self.style.color.r),
                //     pixel.g.min(self.style.color.g),
                //     pixel.b.min(self.style.color.b),
                //     pixel.a,
                // )
                RGBA::new(
                    (((1. - strength) * pixel.r as f64) + (strength * self.style.color.r as f64))
                        as u8,
                    (((1. - strength) * pixel.g as f64) + (strength * self.style.color.g as f64))
                        as u8,
                    (((1. - strength) * pixel.b as f64) + (strength * self.style.color.b as f64))
                        as u8,
                    pixel.a,
                )
            })
            .collect::<Vec<RGBA<u8>>>();

        let new_img = Img::new(new_buf, sub.width(), sub.height());
        let final_img_id = canvas.create_image(new_img.as_ref(), ImageFlags::empty())?;
        Ok(final_img_id)
    }
}

impl Drawable for Highlight {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: femtovg::FontId,
    ) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        let (pos, size) = math::rect_ensure_positive_size(self.top_left, size);
        let scale = canvas.transform().average_scale();
        let dimensions = Vec2D::new(
            canvas.width() as f32 / scale,
            canvas.height() as f32 / scale,
        );
        // we clamp the area to the image dimensions so were not stretching the
        // highlight when the selection exceeds the image size.
        let clamped_pos = Vec2D::new(pos.x.max(0.), pos.y.max(0.));
        let clamped_size = Vec2D::new(
            match pos.x.is_sign_negative() {
                true => size.x - pos.x.abs(),
                false => size.x,
            }
            .min(dimensions.x - clamped_pos.x),
            match pos.y.is_sign_negative() {
                true => size.y - pos.y.abs(),
                false => size.y,
            }
            .min(dimensions.y - clamped_pos.y),
        );
        if self.editing {
            // set style
            let paint =
                Paint::color(self.style.color.into()).with_line_width(Size::Medium.to_line_width());

            // make rect
            let mut path = Path::new();
            path.rect(clamped_pos.x, clamped_pos.y, clamped_size.x, clamped_size.y);

            // draw
            canvas.stroke_path(&path, &paint);
        } else {
            // create new cached image
            if self.cached_image.borrow().is_none() {
                match self.highlight(canvas, clamped_pos, clamped_size) {
                    Ok(hls_image) => self.cached_image.borrow_mut().replace(hls_image),
                    Err(error) => {
                        if error.to_string() == "width or height is 0" {
                            return Ok(());
                        }
                        return Err(error);
                    }
                };
            }

            let mut path = Path::new();
            path.rect(clamped_pos.x, clamped_pos.y, clamped_size.x, clamped_size.y);
            canvas.fill_path(
                &path,
                &Paint::image(
                    self.cached_image.borrow().unwrap(), // this unwrap is safe because we placed it above
                    clamped_pos.x,
                    clamped_pos.y,
                    clamped_size.x,
                    clamped_size.y,
                    0f32,
                    1f32,
                ),
            );
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct HighlightTool {
    highlight: Option<Highlight>,
    style: Style,
}

impl Tool for HighlightTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                // start new
                self.highlight = Some(Highlight {
                    top_left: event.pos,
                    size: None,
                    style: self.style,
                    editing: true,
                    cached_image: RefCell::new(None),
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(a) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        self.highlight = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.size = Some(event.pos);
                        a.editing = false;

                        let result = a.clone_box();
                        self.highlight = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(a) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    a.size = Some(event.pos);

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.highlight.is_some() {
            self.highlight = None;
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
        match &self.highlight {
            Some(d) => Some(d),
            None => None,
        }
    }
}
