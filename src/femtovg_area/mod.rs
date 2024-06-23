mod imp;

use std::{cell::RefCell, rc::Rc};

use gdk_pixbuf::{glib::subclass::types::ObjectSubclassIsExt, Pixbuf};
use gtk::glib;
use relm4::{
    gtk::{self, prelude::WidgetExt},
    Sender,
};

use crate::{
    math::Vec2D,
    sketch_board::{Action, SketchBoardInput},
    tools::{CropTool, Drawable, Tool},
};

glib::wrapper! {
    pub struct FemtoVGArea(ObjectSubclass<imp::FemtoVGArea>)
        @extends gtk::Widget, gtk::GLArea;
}

impl Default for FemtoVGArea {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FemtoVGArea {
    pub fn set_active_tool(&mut self, active_tool: Rc<RefCell<dyn Tool>>) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_active_tool(active_tool);
    }

    pub fn commit(&mut self, drawable: Box<dyn Drawable>) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .commit(drawable);
    }
    pub fn undo(&mut self) -> bool {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .undo()
    }
    pub fn redo(&mut self) -> bool {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .redo()
    }
    pub fn request_render(&self, action: Action) {
        self.imp().request_render(action);
    }

    pub fn abs_canvas_to_image_coordinates(&self, input: Vec2D) -> Vec2D {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .abs_canvas_to_image_coordinates(input, self.scale_factor() as f32)
    }

    pub fn rel_canvas_to_image_coordinates(&self, input: Vec2D) -> Vec2D {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .rel_canvas_to_image_coordinates(input, self.scale_factor() as f32)
    }

    pub fn zoom(&self, factor: f32) {
        self.imp()
            .inner()
            .as_mut()
            .expect("Did you call init before using FemtoVgArea?")
            .set_scale_factor(factor);
    }

    pub fn init(
        &mut self,
        sender: Sender<SketchBoardInput>,
        crop_tool: Rc<RefCell<CropTool>>,
        active_tool: Rc<RefCell<dyn Tool>>,
        background_image: Pixbuf,
    ) {
        self.imp()
            .init(sender, crop_tool, active_tool, background_image);
    }
}
