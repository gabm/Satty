use std::time;

use femtovg::{FontId, Path};
use ink_stroke_modeler_rs::{
    ModelerInput, ModelerInputEventType, ModelerParams, ModelerResult, StrokeModeler,
};

use crate::{
    sketch_board::{MouseButton, MouseEventMsg, MouseEventType},
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
    start_at: time::Instant,
    points: Vec<ModelerInput>,
    style: Style,
}

impl Drawable for BrushDrawable {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> anyhow::Result<()> {
        if self.points.is_empty() {
            return Ok(());
        }

        let mut params = ModelerParams::suggested();
        // params.wobble_smoother_timeout = 0.04;
        // params.wobble_smoother_speed_floor = 1.31;
        // params.wobble_smoother_speed_ceiling = 1.44;
        // params.position_modeler_spring_mass_constant = 11.0 / 32400.0;
        params.position_modeler_drag_constant = 200.0;
        // params.sampling_min_output_rate = 180.0;
        // params.sampling_end_of_stroke_stopping_distance = 0.001;
        // params.sampling_end_of_stroke_max_iterations = 20;
        // params.sampling_max_outputs_per_call = 20;
        // params.stylus_state_modeler_max_input_samples = 10;
        let Ok(mut modeler) = StrokeModeler::new(params) else {
            // TODO:
            return Ok(());
        };

        let result_stroke = self
            .points
            .iter()
            .filter_map(|i| {
                modeler
                    .update(i.clone())
                    .map_err(|e| eprintln!("modeler updated, Err: {e:?}"))
                    .ok()
            })
            .flatten()
            .collect::<Vec<ModelerResult>>();

        if result_stroke.is_empty() {
            return Ok(());
        }

        canvas.save();
        let mut path = Path::new();

        let start_x = self.points[0].pos.0 as f32;
        let start_y = self.points[0].pos.1 as f32;
        path.move_to(start_x, start_y);
        for p in result_stroke.iter().skip(1) {
            path.line_to(start_x + p.pos.0 as f32, start_y + p.pos.1 as f32);
        }
        canvas.stroke_path(&path, &self.style.into());
        canvas.restore();

        Ok(())
    }
}

impl Tool for BrushTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                self.drawable = Some(BrushDrawable {
                    start_at: time::Instant::now(),
                    points: vec![ModelerInput {
                        event_type: ModelerInputEventType::Down,
                        pos: (event.pos.x as f64, event.pos.y as f64),
                        time: 0.,
                        pressure: 0.,
                    }],
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(brush) = &mut self.drawable {
                    // add last point
                    brush.points.push(ModelerInput {
                        event_type: ModelerInputEventType::Up,
                        pos: (event.pos.x as f64, event.pos.y as f64),
                        time: brush.start_at.elapsed().as_secs_f64(),
                        pressure: 0.5,
                    });

                    // commit
                    let result = brush.clone_box();
                    self.drawable = None;

                    ToolUpdateResult::Commit(result)
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(brush) = &mut self.drawable {
                    // add point
                    brush.points.push(ModelerInput {
                        event_type: ModelerInputEventType::Move,
                        pos: (event.pos.x as f64, event.pos.y as f64),
                        time: brush.start_at.elapsed().as_secs_f64(),
                        pressure: 0.5,
                    });

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::Click => {
                // if event.button == MouseButton::Primary {
                //     let brush = Box::new(BrushDrawable {
                //         start_at: time::Instant::now(),
                //         points: vec![],
                //         style: self.style,
                //     });
                //     ToolUpdateResult::Commit(brush)
                // } else {
                ToolUpdateResult::Unmodified
                // }
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
