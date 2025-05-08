use super::{Tool, Tools};

#[derive(Default)]
pub struct PointerTool {
    input_enabled: bool,
}

impl Tool for PointerTool {
    fn get_tool_type(&self) -> super::Tools {
        Tools::Pointer
    }

    fn get_drawable(&self) -> Option<&dyn super::Drawable> {
        None
    }

    fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    fn set_input_enabled(&mut self, value: bool) {
        self.input_enabled = value;
    }
}
