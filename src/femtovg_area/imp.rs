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
    }
    fn render(&self, _context: &gtk::gdk::GLContext) -> bool {
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
                    return false;
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
        false
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

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    pub fn render_native_resolution(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
    ) -> anyhow::Result<ImgVec<RGBA8>> {
        let image_id = canvas.create_image_empty(
            self.background_image.width() as usize,
            self.background_image.height() as usize,
            PixelFormat::Rgba8,
            ImageFlags::empty(),
        )?;

        canvas.set_render_target(femtovg::RenderTarget::Image(image_id));
        canvas.reset_transform();

        self.render(canvas, font, false)?;

        let mut result = canvas.screenshot()?;
        let crop_tool = self.crop_tool.borrow();

        if let Some(crop) = crop_tool.get_crop() {
            if let Some((pos, size)) = crop.get_rectangle() {
                let (buf, width, height) = result
                    .sub_image(
                        pos.x as usize,
                        pos.y as usize,
                        size.x as usize,
                        size.y as usize,
                    )
                    .to_contiguous_buf();
                result = Img::new(buf.to_vec(), width, height);
            }
        }

        Ok(result)
    }

    pub fn render_framebuffer(
        &mut self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
    ) -> Result<()> {
        canvas.set_render_target(femtovg::RenderTarget::Screen);

        // setup transform to image coordinates
        let mut transform = Transform2D::identity();
        let offset_x =
            canvas.width() as f32 - self.background_image.width() as f32 * self.scale_factor;
        let offset_y =
            canvas.height() as f32 - self.background_image.height() as f32 * self.scale_factor;

        transform.scale(self.scale_factor, self.scale_factor);
        transform.translate(offset_x / 2.0, offset_y / 2.0);

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

        unsafe {
            if image.has_alpha() {
                let mut img = Img::new_stride(
                    image.pixels().align_to::<RGBA<u8>>().1.into(),
                    image.width() as usize,
                    image.height() as usize,
                    (image.rowstride() / 4) as usize,
                );

                // this function truncates the internal buffer so that width == stride
                let _ = img.as_contiguous_buf();

                canvas.update_image(background_image_id, ImageSource::Rgba(img.as_ref()), 0, 0)?;
            } else {
                let mut img = Img::new_stride(
                    image.pixels().align_to::<RGB<u8>>().1.into(),
                    image.width() as usize,
                    image.height() as usize,
                    (image.rowstride() / 3) as usize,
                );

                // this function truncates the internal buffer so that width == stride
                let _ = img.as_contiguous_buf();

                canvas.update_image(background_image_id, ImageSource::Rgb(img.as_ref()), 0, 0)?;
            }
        };

        Ok(background_image_id)
    }
}