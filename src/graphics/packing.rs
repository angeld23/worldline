use super::texture::OrientedSection;
use crate::shared::bounding_box::{bbox, BBox2};
use cgmath::{vec2, Array, ElementWise, Vector2};
use linear_map::LinearMap;

#[derive(Debug, Clone)]
pub struct Packer {
    layer_size: Vector2<u32>,
    slots: LinearMap<String, Vector2<u32>>,
    padding: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackedSection {
    pub layer_index: u32,
    pub uv: BBox2,
}

impl From<BBox2> for PackedSection {
    fn from(value: BBox2) -> Self {
        Self {
            layer_index: 0,
            uv: value,
        }
    }
}

impl PackedSection {
    pub fn local_uv(self, local_uv: BBox2) -> Self {
        let [min, size]: [Vector2<f32>; 2] = [self.uv.min().into(), self.uv.size().into()];
        let [local_min, local_max]: [Vector2<f32>; 2] =
            [local_uv.min().into(), local_uv.max().into()];

        Self {
            layer_index: self.layer_index,
            uv: bbox!(
                min + local_min.mul_element_wise(size),
                min + local_max.mul_element_wise(size)
            ),
        }
    }

    pub fn local_point(self, local_point: Vector2<f32>) -> Vector2<f32> {
        let [min, size]: [Vector2<f32>; 2] = [self.uv.min().into(), self.uv.max().into()];
        min + local_point.mul_element_wise(size)
    }

    /// width / height
    pub fn aspect_ratio(self) -> f32 {
        self.uv.size()[0] / self.uv.size()[1]
    }

    pub fn unoriented(self) -> OrientedSection {
        self.into()
    }

    pub fn oriented(self, flipped: bool, clockwise_rotations: u8) -> OrientedSection {
        OrientedSection {
            section: self,
            flipped,
            clockwise_rotations,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackResult {
    pub total_layers: u32,
    pub sections: LinearMap<String, PackedSection>,
}

fn fits(container: Vector2<u32>, inner: Vector2<u32>) -> bool {
    container.x >= inner.x && container.y >= inner.y
}

impl Packer {
    pub fn new(width: u32, height: u32, padding: u32) -> Self {
        Self {
            layer_size: vec2(width, height),
            slots: Default::default(),
            padding,
        }
    }

    pub fn reserve(&mut self, name: impl Into<String>, width: u32, height: u32) -> bool {
        let size = vec2(width, height);
        if fits(self.layer_size, size) {
            self.slots.insert(name.into(), size);
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.slots.clear();
    }

    pub fn pack(self) -> PackResult {
        let mut slots: Vec<(String, Vector2<u32>)> = self.slots.into();
        slots.sort_by(|(_, size_0), (_, size_1)| size_1.product().cmp(&size_0.product()));

        let mut sections = LinearMap::<String, PackedSection>::new();

        let mut current_layer = 0;

        enum Node {
            Open {
                position: Vector2<u32>,
                size: Vector2<u32>,
            },
            Split {
                down: Box<Node>,
                right: Box<Node>,
            },
        }

        impl Node {
            pub fn try_insert(&mut self, slot_size: Vector2<u32>) -> Option<Vector2<u32>> {
                match self {
                    Node::Open { position, size } => {
                        let (position, size) = (*position, *size);
                        if !fits(size, slot_size) {
                            None
                        } else {
                            let down = Node::Open {
                                position: position + vec2(0, slot_size.y),
                                size: size - vec2(0, slot_size.y),
                            };
                            let right = Node::Open {
                                position: position + vec2(slot_size.x, 0),
                                size: vec2(size.x - slot_size.x, slot_size.y),
                            };
                            *self = Node::Split {
                                down: Box::new(down),
                                right: Box::new(right),
                            };
                            Some(position)
                        }
                    }
                    Node::Split { down, right } => right
                        .try_insert(slot_size)
                        .or_else(|| down.try_insert(slot_size)),
                }
            }
        }

        let mut root_node = Node::Open {
            position: vec2(0, 0),
            size: self.layer_size,
        };

        for (name, size) in slots {
            let padded_size = vec2(
                (size.x + self.padding).min(self.layer_size.x),
                (size.y + self.padding).min(self.layer_size.y),
            );

            let position;
            loop {
                match root_node.try_insert(padded_size) {
                    Some(inserted_position) => {
                        position = inserted_position;
                        break;
                    }
                    None => {
                        root_node = Node::Open {
                            position: vec2(0, 0),
                            size: self.layer_size,
                        };
                        current_layer += 1;
                    }
                }
            }

            let uv_0 = vec2(
                position.x as f32 / self.layer_size.x as f32,
                position.y as f32 / self.layer_size.y as f32,
            );
            let pixel_corner = position + size;
            let uv_1 = vec2(
                pixel_corner.x as f32 / self.layer_size.x as f32,
                pixel_corner.y as f32 / self.layer_size.y as f32,
            );

            sections.insert(
                name,
                PackedSection {
                    layer_index: current_layer,
                    uv: bbox!(uv_0, uv_1),
                },
            );
        }

        PackResult {
            total_layers: current_layer + 1,
            sections,
        }
    }
}
