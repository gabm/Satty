use std::f32::consts::PI;

use crate::{
    math::{self, Vec2D},
    sketch_board::{KeyEventMsg, MouseEventMsg, MouseEventType},
};
use anyhow::Result;
use femtovg::{Color, Paint, Path};
use relm4::gtk::gdk::Key;

use super::{Drawable, Tool, ToolUpdateResult};

#[derive(Debug, Clone)]
pub struct Crop {
    pos: Vec2D,
    size: Option<Vec2D>,
    active: bool,
}

#[derive(Default)]
pub struct CropTool {
    crop: Option<Crop>,
    action: Option<CropToolAction>,
}

impl Crop {
    const HANDLE_RADIUS: f32 = 5.0;
    const HANDLE_BORDER: f32 = 2.0;

    fn draw_single_handle(
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        center: Vec2D,
        scale: f32,
    ) {
        let mut path = Path::new();
        path.arc(
            center.x,
            center.y,
            Crop::HANDLE_RADIUS / scale,
            0.0,
            2.0 * PI,
            femtovg::Solidity::Solid,
        );

        let border_paint =
            Paint::color(Color::rgbf(0.9, 0.9, 0.9)).with_line_width(Crop::HANDLE_BORDER / scale);
        let fill_paint = Paint::color(Color::rgbaf(0.0, 0.0, 0.0, 0.4));

        canvas.fill_path(&path, &fill_paint);
        canvas.stroke_path(&path, &border_paint);
    }

    pub fn get_rectangle(&self) -> Option<(Vec2D, Vec2D)> {
        self.size
            .map(|size| math::rect_ensure_positive_size(self.pos, size))
    }
}

impl Drawable for Crop {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: femtovg::FontId,
    ) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        let scale = canvas.transform().average_scale();
        let dimensions = Vec2D::new(
            canvas.width() as f32 / scale,
            canvas.height() as f32 / scale,
        );

        let shadow_paint = Paint::color(Color::rgbaf(0.0, 0.0, 0.0, 0.5))
            .with_fill_rule(femtovg::FillRule::EvenOdd);
        let mut shadow_path = Path::new();
        shadow_path.rect(0.0, 0.0, dimensions.x, dimensions.y);
        shadow_path.rect(self.pos.x, self.pos.y, size.x, size.y);

        let border_paint = Paint::color(Color::rgbf(0.1, 0.1, 0.1)).with_line_width(2.0);
        let mut border_path = Path::new();
        border_path.rect(self.pos.x, self.pos.y, size.x, size.y);

        canvas.save();
        canvas.fill_path(&shadow_path, &shadow_paint);
        canvas.stroke_path(&border_path, &border_paint);

        if self.active {
            Self::draw_single_handle(canvas, self.pos, scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(size.x / 2.0, 0.0), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(size.x, 0.0), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(0.0, size.y / 2.0), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(0.0, size.y), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(size.x / 2.0, size.y), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(size.x, size.y), scale);
            Self::draw_single_handle(canvas, self.pos + Vec2D::new(size.x, size.y / 2.0), scale);
        }

        canvas.restore();
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum CropHandle {
    TopLeftCorner,
    TopEdge,
    TopRightCorner,
    RightEdge,
    BottomRightCorner,
    BottomEdge,
    BottomLeftCorner,
    LeftEdge,
}

enum CropToolAction {
    NewCrop,
    DragHandle(DragHandleState),
    Move(MoveState),
}

struct DragHandleState {
    handle: CropHandle,
    top_left_start: Vec2D,
    bottom_right_start: Vec2D,
}

struct MoveState {
    start: Vec2D,
}

impl CropTool {
    pub fn get_crop(&self) -> Option<&Crop> {
        match &self.crop {
            Some(c) => Some(c),
            None => None,
        }
    }
}

impl CropHandle {
    fn all() -> [CropHandle; 8] {
        [
            CropHandle::TopLeftCorner,
            CropHandle::TopEdge,
            CropHandle::TopRightCorner,
            CropHandle::RightEdge,
            CropHandle::BottomRightCorner,
            CropHandle::BottomEdge,
            CropHandle::BottomLeftCorner,
            CropHandle::LeftEdge,
        ]
    }
}

impl CropTool {
    fn get_handle_pos(crop_pos: Vec2D, crop_size: Vec2D, handle: CropHandle) -> Vec2D {
        match handle {
            CropHandle::TopLeftCorner => crop_pos,
            CropHandle::TopEdge => crop_pos + Vec2D::new(crop_size.x / 2.0, 0.0),
            CropHandle::TopRightCorner => crop_pos + Vec2D::new(crop_size.x, 0.0),
            CropHandle::RightEdge => crop_pos + Vec2D::new(crop_size.x, crop_size.y / 2.0),
            CropHandle::BottomRightCorner => crop_pos + Vec2D::new(crop_size.x, crop_size.y),
            CropHandle::BottomEdge => crop_pos + Vec2D::new(crop_size.x / 2.0, crop_size.y),
            CropHandle::BottomLeftCorner => crop_pos + Vec2D::new(0.0, crop_size.y),
            CropHandle::LeftEdge => crop_pos + Vec2D::new(0.0, crop_size.y / 2.0),
        }
    }
    fn test_handle_hit(&self, mouse_pos: Vec2D) -> Option<(CropHandle, Vec2D, Vec2D)> {
        let crop = self.crop.as_ref()?;

        let crop_size = crop.size?;
        let crop_pos = crop.pos;

        const MAX_DISTANCE2: f32 = (Crop::HANDLE_BORDER + Crop::HANDLE_RADIUS)
            * (Crop::HANDLE_RADIUS + Crop::HANDLE_BORDER);

        for h in CropHandle::all() {
            if (Self::get_handle_pos(crop_pos, crop_size, h) - mouse_pos).norm2() < MAX_DISTANCE2 {
                return Some((h, crop_pos, crop_size));
            }
        }
        None
    }

    fn test_inside_crop(&self, mouse_pos: Vec2D) -> bool {
        let crop = match &self.crop {
            Some(c) => c,
            None => return false,
        };

        let crop_size = match crop.size {
            Some(s) => s,
            None => return false,
        };

        let (mut min_x, mut max_x) = (crop.pos.x, crop.pos.x + crop_size.x);
        if min_x > max_x {
            (min_x, max_x) = (max_x, min_x);
        }

        let (mut min_y, mut max_y) = (crop.pos.y, crop.pos.y + crop_size.y);
        if min_y > max_y {
            (min_y, max_y) = (max_y, min_y);
        }

        min_x < mouse_pos.x && mouse_pos.x < max_x && min_y < mouse_pos.y && mouse_pos.y < max_y
    }

    fn apply_drag_handle_transformation(
        crop: &mut Crop,
        state: &DragHandleState,
        direction: Vec2D,
    ) {
        let mut tl = state.top_left_start;
        let mut br = state.bottom_right_start;

        // apply transformation
        match state.handle {
            CropHandle::TopLeftCorner => {
                tl += direction;
            }
            CropHandle::TopEdge => {
                tl += Vec2D::new(0.0, direction.y);
            }
            CropHandle::TopRightCorner => {
                tl += Vec2D::new(0.0, direction.y);
                br += Vec2D::new(direction.x, 0.0);
            }
            CropHandle::RightEdge => {
                br += Vec2D::new(direction.x, 0.0);
            }
            CropHandle::BottomRightCorner => {
                br += direction;
            }
            CropHandle::BottomEdge => {
                br += Vec2D::new(0.0, direction.y);
            }
            CropHandle::BottomLeftCorner => {
                tl += Vec2D::new(direction.x, 0.0);
                br += Vec2D::new(0.0, direction.y);
            }
            CropHandle::LeftEdge => {
                tl += Vec2D::new(direction.x, 0.0);
            }
        }

        // convert back and save
        crop.pos = tl;
        crop.size = Some(br - tl);
    }

    fn begin_drag(&mut self, pos: Vec2D) -> ToolUpdateResult {
        if let Some((handle, pos, size)) = self.test_handle_hit(pos) {
            let top_left_start = pos;
            let bottom_right_start = pos + size;
            self.action = Some(CropToolAction::DragHandle(DragHandleState {
                handle,
                top_left_start,
                bottom_right_start,
            }));
        } else {
            // only start a new crop if none exists
            match &self.crop {
                None => {
                    self.crop = Some(Crop {
                        pos,
                        size: None,
                        active: true,
                    });
                    self.action = Some(CropToolAction::NewCrop);
                }
                Some(c) => {
                    if self.test_inside_crop(pos) {
                        self.action = Some(CropToolAction::Move(MoveState { start: c.pos }));
                    }
                }
            }
        }
        ToolUpdateResult::Redraw
    }

    fn update_drag(&mut self, direction: Vec2D) -> ToolUpdateResult {
        let crop = match &mut self.crop {
            Some(c) => c,
            None => return ToolUpdateResult::Unmodified,
        };

        let action = match &self.action {
            Some(a) => a,
            None => return ToolUpdateResult::Unmodified,
        };

        match action {
            CropToolAction::NewCrop => {
                crop.size = Some(direction);
                ToolUpdateResult::Redraw
            }
            CropToolAction::DragHandle(state) => {
                Self::apply_drag_handle_transformation(crop, state, direction);
                ToolUpdateResult::Redraw
            }
            CropToolAction::Move(state) => {
                crop.pos = state.start + direction;
                ToolUpdateResult::Redraw
            }
        }
    }

    fn end_drag(&mut self, direction: Vec2D) -> ToolUpdateResult {
        let Some(crop) = &mut self.crop else {
            return ToolUpdateResult::Unmodified;
        };

        let Some(action) = &self.action else {
            return ToolUpdateResult::Unmodified;
        };

        match action {
            // crop never returns "commit" because nothing gets
            // commited to the drawables stack
            CropToolAction::NewCrop => {
                crop.size = Some(direction);
                self.action = None;
                ToolUpdateResult::Redraw
            }
            CropToolAction::DragHandle(state) => {
                Self::apply_drag_handle_transformation(crop, state, direction);
                self.action = None;
                ToolUpdateResult::Redraw
            }
            CropToolAction::Move(state) => {
                crop.pos = state.start + direction;
                self.action = None;
                ToolUpdateResult::Redraw
            }
        }
    }
}

impl Tool for CropTool {
    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.crop.is_some() {
            self.handle_deactivated()
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => self.begin_drag(event.pos),
            MouseEventType::EndDrag => self.end_drag(event.pos),
            MouseEventType::UpdateDrag => self.update_drag(event.pos),
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_activated(&mut self) -> ToolUpdateResult {
        if let Some(c) = &mut self.crop {
            c.active = true;
            return ToolUpdateResult::Redraw;
        }
        ToolUpdateResult::Unmodified
    }

    fn handle_deactivated(&mut self) -> ToolUpdateResult {
        if let Some(c) = &mut self.crop {
            c.active = false;
        }
        self.action = None;
        ToolUpdateResult::Redraw
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        // the reason we always return None is because we dont want this tool
        // to show up with the standard rendering mechanism. Instead it will always
        // be drawn seperately by using `get_crop(&self)`
        None
    }
}
