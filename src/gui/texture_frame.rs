use super::{
    color::GuiColor,
    element::{GuiContext, GuiElement, GuiPrimitive},
    transform::GuiTransform,
};
use crate::graphics::texture::OrientedSection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureFrame {
    pub transform: GuiTransform,
    pub color: GuiColor,
    pub section: OrientedSection,
}

impl GuiElement for TextureFrame {
    fn transform(&self) -> GuiTransform {
        self.transform
    }

    fn render(&self, context: &mut GuiContext) -> Vec<GuiPrimitive> {
        let GuiContext { frame, .. } = context;
        let frame = *frame;

        vec![GuiPrimitive {
            absolute_position: self.transform.absolute_position(frame),
            absolute_size: self.transform.absolute_size(frame),
            section: self.section,
            color: self.color,
        }]
    }
}
