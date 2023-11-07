use super::Tool;

#[derive(Default)]
pub struct PointerTool {}

impl Tool for PointerTool {
    fn get_drawable(&self) -> Option<&dyn super::Drawable> {
        None
    }
}
