use super::element::{GuiContext, GuiElement};
use crate::{graphics::vertex::Vertex2D, shared::indexed_container::IndexedContainer};

#[derive(Debug)]
pub struct GuiBuilder<'a> {
    vertices: IndexedContainer<Vertex2D>,
    pub context: GuiContext<'a>,
}

impl<'a> GuiBuilder<'a> {
    pub fn new(context: GuiContext<'a>) -> Self {
        Self {
            vertices: Default::default(),
            context,
        }
    }

    pub fn element(&mut self, element: impl GuiElement) -> &mut Self {
        let primitives = element.render(&mut self.context);

        self.vertices.items.reserve(primitives.len() * 4);
        self.vertices.indices.reserve(primitives.len() * 6);
        for mut primitive in primitives {
            primitive.absolute_position += self.context.offset;
            self.vertices
                .push_container(primitive.vertices(self.context.frame));
        }
        self
    }

    pub fn element_children(
        &mut self,
        element: impl GuiElement,
        mut children: impl FnMut(&mut Self),
    ) -> &mut Self {
        let old_frame = self.context.frame;
        let old_offset = self.context.offset;

        let transform = element.transform();

        self.element(element);

        self.context.frame = transform.absolute_size(old_frame);
        self.context.offset = old_offset + transform.absolute_position(old_frame);

        children(self);

        self.context.frame = old_frame;
        self.context.offset = old_offset;

        self
    }

    pub fn finish(self) -> IndexedContainer<Vertex2D> {
        self.vertices
    }
}
