use std::collections::BTreeMap;

use super::vertex::Vertex3D;
use crate::shared::{
    f32_util::IsSmall,
    indexed_container::{IndexedContainer, IndexedVertices},
};
use cgmath::{vec3, InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector3};
use include_dir::include_dir;
use lazy_static::lazy_static;
use obj::ObjData;

/// A shape that is part of a model.
#[derive(Debug, Clone)]
pub enum Shape {
    RawMesh {
        vertices: IndexedContainer<Vertex3D>,
    },
}

impl Shape {
    pub fn generate_mesh(
        &self,
        output_container: &mut IndexedContainer<Vertex3D>,
        transform: Matrix4<f32>,
    ) {
        match self {
            Shape::RawMesh { vertices } => {
                if transform.is_identity() {
                    output_container.push_relative_indexed(
                        vertices.items.iter().copied(),
                        vertices.indices.iter().copied(),
                    );
                } else {
                    let rotation = Matrix3::from_cols(
                        transform.x.truncate(),
                        transform.y.truncate(),
                        transform.z.truncate(),
                    );

                    output_container.items.reserve(vertices.items.len());
                    output_container.indices.reserve(vertices.indices.len());

                    let index_offset = output_container.items.len() as u32;

                    for &vertex in vertices.items.iter() {
                        let normal = rotation * Vector3::from(vertex.normal);
                        let normal = if normal.is_small() {
                            vec3(1.0, 0.0, 0.0)
                        } else {
                            normal.normalize()
                        };
                        output_container.push(Vertex3D {
                            pos: (transform * Vector3::from(vertex.pos).extend(1.0))
                                .truncate()
                                .into(),
                            normal: normal.into(),
                            ..vertex
                        })
                    }

                    for &index in vertices.indices.iter() {
                        output_container.indices.push(index + index_offset);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Model {
    pub vertices: IndexedVertices<Vertex3D>,
}

lazy_static! {
    pub static ref MODEL_DATA: BTreeMap<String, ObjData> = {
        const MODEL_DIR: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/src/graphics/models");

        fn extract_files<'a>(
            out: &mut Vec<include_dir::File<'a>>,
            entry: include_dir::DirEntry<'a>,
        ) {
            match entry {
                include_dir::DirEntry::Dir(dir) => {
                    for child_entry in dir.entries() {
                        extract_files(out, child_entry.to_owned());
                    }
                }
                include_dir::DirEntry::File(file) => out.push(file),
            }
        }

        let mut files = Vec::<include_dir::File>::new();
        for entry in MODEL_DIR.entries() {
            extract_files(&mut files, entry.to_owned());
        }

        let mut model_data = BTreeMap::new();

        for file in files {
            if let Ok(data) = ObjData::load_buf(file.contents()) {
                model_data.insert(
                    file.path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    data,
                );
            }
        }

        model_data
    };
}
