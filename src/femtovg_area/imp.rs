use anyhow::Result;
use glow::HasContext;
use std::{
    cell::{RefCell, RefMut},
    num::NonZeroU32,
    rc::Rc,
};

use femtovg::{
    imgref::{Img, ImgVec},
    renderer,
    rgb::{RGB, RGBA, RGBA8},
    Canvas, FontId, ImageFlags, ImageId, ImageSource, Paint, Path, PixelFormat, Transform2D,
};
use gdk_pixbuf::Pixbuf;
use gtk::{glib, prelude::*, subclass::prelude::*};
use relm4::{gtk, Sender};
use resource::resource;

use crate::{
    math::Vec2D,
    sketch_board::{Action, SketchBoardInput},
    tools::{CropTool, Drawable, Tool},
};

#[derive(Default)]
pub struct FemtoVGArea {
    canvas: RefCell<Option<femtovg::Canvas<femtovg::renderer::OpenGl>>>,
    font: RefCell<Option<FontId>>,
    inner: RefCell<Option<FemtoVgAreaMut>>,
    request_render: RefCell<Option<Action>>,
    sender: RefCell<Option<Sender<SketchBoardInput>>>,
}

pub struct FemtoVgAreaMut {
    background_image: Pixbuf,
    background_image_id: Option<femtovg::ImageId>,
    active_tool: Rc<RefCell<dyn Tool>>,
    crop_tool: Rc<RefCell<CropTool>>,
    scale_factor: f32,
    offset: Vec2D,
    drawables: Vec<Box<dyn Drawable>>,
    redo_stack: Vec<Box<dyn Drawable>>,
}

#[glib::object_subclass]
impl ObjectSubclass for FemtoVGArea {
    const NAME: &'static str = "FemtoVGArea";
    type Type = super::FemtoVGArea;
    type ParentType = gtk::GLArea;
}

impl ObjectImpl for FemtoVGArea {
    fn constructed(&self) {
        self.parent_constructed();
        let area = self.obj();
        area.set_has_stencil_buffer(true);
        area.queue_render();
    }
}

impl WidgetImpl for FemtoVGArea {
    fn realize(&self) {
        self.parent_realize();
    }
    fn unrealize(&self) {
        self.obj().make_current();
        self.canvas.borrow_mut().take();
        self.parent_unrealize();
    }
}

impl GLAreaImpl for FemtoVGArea {
    fn resize(&self, width: i32, height: i32) {
        self.ensure_canvas();

        let mut bc = self.canvas.borrow_mut();
        let canvas = bc.as_mut().unwrap(); // this unwrap is safe as long as we call "ensure_canvas" before

        canvas.set_size(
            width as u32,
            height as u32,
            self.obj().scale_factor() as f32,
        );

        // update scale factor
        self.inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .update_transformation(canvas);
    }
    fn render(&self, _context: &gtk::gdk::GLContext) -> glib::Propagation {
        self.ensure_canvas();

        let mut bc = self.canvas.borrow_mut();
        let canvas = bc.as_mut().unwrap(); // this unwrap is safe as long as we call "ensure_canvas" before
        let font = self.font.borrow().unwrap(); // this unwrap is safe as long as we call "ensure_canvas" before
        let mut action = self.request_render.borrow_mut();

        // if we got requested to render a frame
        if let Some(a) = action.as_ref() {
            // render image
            let image = match self
                .inner()
                .as_mut()
                .expect("Did you call init before using FemtoVgArea?")
                .render_native_resolution(canvas, font)
            {
                Ok(t) => t,
                Err(e) => {
                    println!("Error while rendering image: {e}");
                    return glib::Propagation::Stop;
                }
            };

            // send result
            self.sender
                .borrow()
                .as_ref()
                .expect("Did you call init before using FemtoVgArea?")
                .emit(SketchBoardInput::RenderResult(image, *a));

            // reset request
            *action = None;
        }
        if let Err(e) = self
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .render_framebuffer(canvas, font)
        {
            println!("Error rendering to framebuffer: {e}");
        }
        glib::Propagation::Stop
    }
}
impl FemtoVGArea {
    pub fn init(
        &self,
        sender: Sender<SketchBoardInput>,
        crop_tool: Rc<RefCell<CropTool>>,
        active_tool: Rc<RefCell<dyn Tool>>,
        background_image: Pixbuf,
    ) {
        self.inner().replace(FemtoVgAreaMut {
            background_image,
            background_image_id: None,
            active_tool,
            crop_tool,
            scale_factor: 1.0,
            offset: Vec2D::zero(),
            drawables: Vec::new(),
            redo_stack: Vec::new(),
        });
        self.sender.borrow_mut().replace(sender);
    }
    fn ensure_canvas(&self) {
        if self.canvas.borrow().is_none() {
            let c = self
                .setup_canvas()
                .expect("Cannot setup renderer and canvas");
            self.canvas.borrow_mut().replace(c);
        }

        self.font.borrow_mut().replace(
            self.canvas
                .borrow_mut()
                .as_mut()
                .unwrap() // this unwrap is safe because it gets placed above
                .add_font_mem(&resource!("src/assets/Roboto-Regular.ttf"))
                .expect("Cannot add font"),
        );
    }

    fn setup_canvas(&self) -> Result<femtovg::Canvas<femtovg::renderer::OpenGl>> {
        let widget = self.obj();
        widget.attach_buffers();

        static LOAD_FN: fn(&str) -> *const std::ffi::c_void =
            |s| epoxy::get_proc_addr(s) as *const _;
        // SAFETY: Need to get the framebuffer id that gtk expects us to draw into, so
        // femtovg knows which framebuffer to bind. This is safe as long as we
        // call attach_buffers beforehand. Also unbind it here just in case,
        // since this can be called outside render.
        let (mut renderer, fbo) = unsafe {
            let renderer =
                renderer::OpenGl::new_from_function(LOAD_FN).expect("Cannot create renderer");
            let ctx = glow::Context::from_loader_function(LOAD_FN);
            let id = NonZeroU32::new(ctx.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32)
                .expect("No GTK provided framebuffer binding");
            ctx.bind_framebuffer(glow::FRAMEBUFFER, None);
            (renderer, glow::NativeFramebuffer(id))
        };
        renderer.set_screen_target(Some(fbo));
        Ok(Canvas::new(renderer)?)
    }

    pub fn inner(&self) -> RefMut<'_, Option<FemtoVgAreaMut>> {
        self.inner.borrow_mut()
    }
    pub fn request_render(&self, action: Action) {
        self.request_render.borrow_mut().replace(action);
        self.obj().queue_render();
    }
    pub fn set_parent_sender(&self, sender: Sender<SketchBoardInput>) {
        self.sender.borrow_mut().replace(sender);
    }
}

impl FemtoVgAreaMut {
    pub fn commit(&mut self, drawable: Box<dyn Drawable>) {
        self.drawables.push(drawable);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        match self.drawables.pop() {
            Some(mut d) => {
                // notify of the undo action
                d.handle_undo();

                // push to redo stack
                self.redo_stack.push(d);
                true
            }
            None => false,
        }
    }
    pub fn redo(&mut self) -> bool {
        match self.redo_stack.pop() {
            Some(mut d) => {
                // notify of the redo action
                d.handle_redo();

                // push to drawable stack
                self.drawables.push(d);

                true
            }
            None => false,
        }
    }

    pub fn set_active_tool(&mut self, active_tool: Rc<RefCell<dyn Tool>>) {
        self.active_tool = active_tool;
    }

    pub fn render_native_resolution(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
    ) -> anyhow::Result<ImgVec<RGBA8>> {
        // get offset and size of the area in question
        let (pos, size) = self
            .crop_tool
            .borrow()
            .get_crop()
            .and_then(|c| c.get_rectangle())
            .unwrap_or((
                Vec2D::zero(),
                Vec2D::new(
                    self.background_image.width() as f32,
                    self.background_image.height() as f32,
                ),
            ));

        // create render-target
        let image_id = canvas.create_image_empty(
            size.x as usize,
            size.y as usize,
            PixelFormat::Rgba8,
            ImageFlags::empty(),
        )?;
        canvas.set_render_target(femtovg::RenderTarget::Image(image_id));

        // apply offset
        let mut transform = Transform2D::identity();
        transform.translate(-pos.x, -pos.y);
        canvas.reset_transform();
        canvas.set_transform(&transform);

        // render
        self.render(canvas, font, false)?;

        // return screenshot
        let result = canvas.screenshot();

        // clean up
        canvas.set_render_target(femtovg::RenderTarget::Screen);
        canvas.delete_image(image_id);

        Ok(result?)
    }

    pub fn render_framebuffer(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
    ) -> Result<()> {
        canvas.set_render_target(femtovg::RenderTarget::Screen);

        // setup transform to image coordinates
        let mut transform = Transform2D::identity();
        transform.scale(self.scale_factor, self.scale_factor);
        transform.translate(self.offset.x, self.offset.y);

        canvas.reset_transform();
        canvas.set_transform(&transform);

        self.render(canvas, font, true)?;

        Ok(())
    }

    fn render(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
        render_crop: bool,
    ) -> Result<()> {
        // clear canvas
        canvas.clear_rect(
            0,
            0,
            canvas.width(),
            canvas.height(),
            femtovg::Color::black(),
        );

        // render background
        self.render_background_image(canvas)?;

        // render the whole stack
        for d in &mut self.drawables {
            d.draw(canvas, font)?;
        }

        // render active tool
        if let Some(d) = self.active_tool.borrow().get_drawable() {
            d.draw(canvas, font)?;
        }

        // render crop tool
        if render_crop {
            if let Some(c) = self.crop_tool.borrow().get_crop() {
                c.draw(canvas, font)?;
            }
        }

        canvas.flush();
        Ok(())
    }

    fn render_background_image(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
    ) -> Result<()> {
        let background_image_id = match self.background_image_id {
            Some(id) => id,
            None => {
                let id = Self::upload_background_image(canvas, &self.background_image)?;
                self.background_image_id.replace(id);
                id
            }
        };

        // render the image
        let mut path = Path::new();
        path.rect(
            0.0,
            0.0,
            self.background_image.width() as f32,
            self.background_image.height() as f32,
        );

        canvas.fill_path(
            &path,
            &Paint::image(
                background_image_id,
                0f32,
                0f32,
                self.background_image.width() as f32,
                self.background_image.height() as f32,
                0f32,
                1f32,
            ),
        );

        Ok(())
    }

    fn upload_background_image(
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        image: &Pixbuf,
    ) -> Result<ImageId> {
        let format = if image.has_alpha() {
            PixelFormat::Rgba8
        } else {
            PixelFormat::Rgb8
        };

        let background_image_id = canvas.create_image_empty(
            image.width() as usize,
            image.height() as usize,
            format,
            ImageFlags::empty(),
        )?;

        // extract values
        let width = image.width() as usize;
        let stride = image.rowstride() as usize; // stride is in bytes per row
        let height = image.height() as usize;
        let bytes_per_pixel = if image.has_alpha() { 4 } else { 3 }; // pixbuf supports rgb or rgba

        unsafe {
            let src_buffer = image.pixels();

            let row_length = width * bytes_per_pixel;
            let mut dst_buffer = if row_length == stride {
                // stride == row_length, there are no additional bytes after the end of each row
                src_buffer.to_vec()
            } else {
                // stride != row_length, there are additional bytes after the end of each row that
                // need to be truncated. We copy row by row..
                let mut dst_buffer = Vec::<u8>::with_capacity(width * height * bytes_per_pixel);

                for row in 0..height {
                    let src_offset = row * stride;
                    dst_buffer.extend_from_slice(&src_buffer[src_offset..src_offset + row_length]);
                }
                dst_buffer
            };

            // in almost all cases, that should be a no-op. Buf we might have additional elements after the
            // end of the buffer, e.g. after width * height * bytes_per_pixel
            dst_buffer.truncate(width * height * bytes_per_pixel);

            if image.has_alpha() {
                let img = Img::new_stride(
                    dst_buffer.align_to::<RGBA<u8>>().1.to_vec(),
                    width,
                    height,
                    width,
                );

                canvas.update_image(background_image_id, ImageSource::Rgba(img.as_ref()), 0, 0)?;
            } else {
                let img = Img::new_stride(
                    dst_buffer.align_to::<RGB<u8>>().1.to_owned(),
                    width,
                    height,
                    width,
                );

                canvas.update_image(background_image_id, ImageSource::Rgb(img.as_ref()), 0, 0)?;
            }
        }

        Ok(background_image_id)
    }

    pub fn update_transformation(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
    ) {
        // calculate scale
        let image_width = self.background_image.width() as f32;
        let image_height = self.background_image.height() as f32;
        let aspect_ratio = image_width / image_height;

        let canvas_width = canvas.width() as f32;
        let canvas_height = canvas.height() as f32;

        self.scale_factor = if canvas_width / aspect_ratio <= canvas_height {
            canvas_width / aspect_ratio / image_height
        } else {
            canvas_height * aspect_ratio / image_width
        };

        // calculate offset
        self.offset = Vec2D::new(
            (canvas.width() as f32 - self.background_image.width() as f32 * self.scale_factor)
                / 2.0,
            (canvas.height() as f32 - self.background_image.height() as f32 * self.scale_factor)
                / 2.0,
        );
    }

    pub fn abs_canvas_to_image_coordinates(&self, input: Vec2D, dpi_scale_factor: f32) -> Vec2D {
        Vec2D::new(
            (input.x * dpi_scale_factor - self.offset.x) / self.scale_factor,
            (input.y * dpi_scale_factor - self.offset.y) / self.scale_factor,
        )
    }
    pub fn rel_canvas_to_image_coordinates(&self, input: Vec2D, dpi_scale_factor: f32) -> Vec2D {
        Vec2D::new(
            input.x * dpi_scale_factor / self.scale_factor,
            input.y * dpi_scale_factor / self.scale_factor,
        )
    }
}
