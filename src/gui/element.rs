use super::{builder::GuiBuilder, color::GuiColor, text::TextLabel, transform::GuiTransform};
use crate::{
    app_state::TextureProvider,
    graphics::{texture::OrientedSection, vertex::Vertex2D},
    shared::{bounding_box::bbox, indexed_container::IndexedContainer, input::InputController},
};
use cgmath::{vec2, ElementWise, Vector2};

#[derive(Debug)]
pub struct GuiContext<'a> {
    pub frame: Vector2<f32>,
    pub global_frame: Vector2<f32>,
    pub offset: Vector2<f32>,

    pub texture_provider: &'a TextureProvider,
    pub input_controller: &'a mut InputController,
}

impl<'a> GuiContext<'a> {
    pub fn new(
        frame: Vector2<f32>,
        texture_provider: &'a TextureProvider,
        input_controller: &'a mut InputController,
    ) -> Self {
        Self {
            frame,
            global_frame: frame,
            offset: vec2(0.0, 0.0),

            texture_provider,
            input_controller,
        }
    }

    pub fn builder(self) -> GuiBuilder<'a> {
        GuiBuilder::new(self)
    }

    pub fn absolute_position(&self, transform: GuiTransform) -> Vector2<f32> {
        transform.absolute_position(self.frame) + self.offset
    }

    pub fn absolute_size(&self, transform: GuiTransform) -> Vector2<f32> {
        transform.absolute_size(self.frame)
    }

    /// (absolute_position, absolute_size)
    pub fn absolute(&self, transform: GuiTransform) -> (Vector2<f32>, Vector2<f32>) {
        (
            self.absolute_position(transform),
            self.absolute_size(transform),
        )
    }

    pub fn white(&self) -> OrientedSection {
        self.texture_provider.get_section("white")
    }

    pub fn char_pixel_height(&self, transform: GuiTransform, lines: u32) -> f32 {
        TextLabel::get_max_char_pixel_height(self.absolute_size(transform).y, lines)
    }
}

pub trait GuiElement {
    fn transform(&self) -> GuiTransform;
    fn render(&self, context: &mut GuiContext) -> Vec<GuiPrimitive>;
}

#[derive(Debug, Clone, Copy)]
pub struct GuiPrimitive {
    pub absolute_position: Vector2<f32>,
    pub absolute_size: Vector2<f32>,
    pub section: OrientedSection,
    pub color: GuiColor,
}

impl GuiPrimitive {
    pub fn vertices(&self, frame: Vector2<f32>) -> IndexedContainer<Vertex2D> {
        if !self.color.is_visible() {
            return IndexedContainer::default();
        }

        let corner_0 = self.absolute_position.div_element_wise(frame);
        let corner_1 = corner_0 + self.absolute_size.div_element_wise(frame);
        let rect = bbox!(corner_0, corner_1);

        let color = [self.color.r, self.color.g, self.color.b, self.color.a];

        let uv = self.section.uv_corners();
        let tex_index = self.section.section.layer_index;

        IndexedContainer {
            items: vec![
                Vertex2D {
                    pos: rect.get_corner([false, false]),
                    uv: uv.top_left,
                    tex_index,
                    color,
                },
                Vertex2D {
                    pos: rect.get_corner([false, true]),
                    uv: uv.bottom_left,
                    tex_index,
                    color,
                },
                Vertex2D {
                    pos: rect.get_corner([true, true]),
                    uv: uv.bottom_right,
                    tex_index,
                    color,
                },
                Vertex2D {
                    pos: rect.get_corner([true, false]),
                    uv: uv.top_right,
                    tex_index,
                    color,
                },
            ],
            indices: vec![0, 1, 2, 2, 3, 0],
        }
    }
}
