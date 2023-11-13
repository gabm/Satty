use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use gdk_pixbuf::glib::Bytes;
use gdk_pixbuf::Pixbuf;
use pangocairo::cairo::{Context, Format, ImageSurface, Operator};

use relm4::gtk::gdk::{MemoryFormat, MemoryTexture};
use relm4::gtk::prelude::*;

use crate::tools::CropTool;
use crate::tools::Drawable;
use crate::tools::Tool;

pub struct Renderer {
    original_image: Pixbuf,
    crop_tool: Rc<RefCell<CropTool>>,
    drawables: Vec<Box<dyn Drawable>>,
    redo_stack: Vec<Box<dyn Drawable>>,
}

impl Renderer {
    pub fn new(original_image: Pixbuf, crop_tool: Rc<RefCell<CropTool>>) -> Self {
        Self {
            original_image,
            drawables: Vec::new(),
            redo_stack: Vec::new(),
            crop_tool,
        }
    }

    pub fn render_full_size(
        &self,
        render_crop: bool,
        active_tool: &Rc<RefCell<dyn Tool>>,
    ) -> Result<ImageSurface> {
        let surface = ImageSurface::create(
            Format::ARgb32,
            self.original_image.width(),
            self.original_image.height(),
        )?;

        let cx: Context = Context::new(surface.clone())?;
        cx.set_operator(Operator::Over);

        // render background image
        cx.set_source_pixbuf(&self.original_image, 0.0, 0.0);
        cx.paint()?;

        // render comitted drawables
        for da in &self.drawables {
            da.draw(&cx, &surface)?;
        }

        // render drawable of active tool, if any
        if let Some(d) = &active_tool.borrow().get_drawable() {
            d.draw(&cx, &surface)?;
        }

        if render_crop {
            // render crop (even if tool not active)
            if let Some(c) = self.crop_tool.borrow().get_crop() {
                c.draw(&cx, &surface)?;
            }
        }

        Ok(surface)
    }

    pub fn render_to_window(
        &self,
        cx: &Context,
        scale_factor: f64,
        active_tool: &Rc<RefCell<dyn Tool>>,
    ) -> Result<()> {
        let surface = self.render_full_size(true, active_tool)?;

        // render to window
        cx.scale(scale_factor, scale_factor);
        cx.set_source_surface(surface, 0.0, 0.0)?;
        cx.paint()?;

        Ok(())
    }

    pub fn render_with_crop(&self, active_tool: &Rc<RefCell<dyn Tool>>) -> Result<ImageSurface> {
        // render final image
        let mut surface = self.render_full_size(false, active_tool)?;

        if let Some((pos, size)) = self
            .crop_tool
            .borrow()
            .get_crop()
            .and_then(|c| c.get_rectangle())
        {
            // crop the full size render to target values
            let cropped_surface =
                ImageSurface::create(Format::ARgb32, size.x as i32, size.y as i32)?;
            let cropped_cx = Context::new(cropped_surface.clone())?;
            cropped_cx.set_source_surface(surface, -pos.x, -pos.y)?;
            cropped_cx.paint()?;

            surface = cropped_surface.clone();
        }

        Ok(surface)
    }

    pub fn render_to_pixbuf(&self, active_tool: &Rc<RefCell<dyn Tool>>) -> Result<Pixbuf> {
        let mut surface = self.render_with_crop(active_tool)?;
        let height = surface.height();
        let width = surface.width();
        let stride = surface.stride();
        let data = surface.data()?;

        Ok(Pixbuf::from_bytes(
            &Bytes::from(&*data),
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width,
            height,
            stride,
        ))
    }

    pub fn render_to_texture(&self, active_tool: &Rc<RefCell<dyn Tool>>) -> Result<MemoryTexture> {
        let mut surface = self.render_with_crop(active_tool)?;

        let height = surface.height();
        let width = surface.width();
        let stride = surface.stride() as usize;
        let data = surface.data()?;

        let texture = MemoryTexture::new(
            width,
            height,
            MemoryFormat::B8g8r8a8Premultiplied,
            &Bytes::from(&*data),
            stride,
        );

        Ok(texture)
    }

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
}
