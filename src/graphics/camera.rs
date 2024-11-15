use cgmath::{vec3, Deg, Matrix4, Quaternion, SquareMatrix, Vector3};

#[rustfmt::skip]
/// Since cgmath uses OpenGL's NDC space which has a range of [-1.0, +1.0] for the z-axis, but wgpu uses [0.0, +1.0],
/// it's best to convert any cgmath results to wgpu's NDC space before passing them to wgpu. This prevents unexpected clipping.
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    // NOTE: Matrix4::new() takes the matrix in column-major order so the formatting of the values here is a bit misleading
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0, 
);

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_projection: [[f32; 4]; 4],
    pub _padding: [u32; 3], // this is the worst thing on the planet
    pub aspect_ratio: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub vertical_fov: Deg<f32>,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: vec3(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            vertical_fov: Deg(40.0),
            near_plane: 0.001,
            far_plane: 15000.0,
        }
    }
}

impl Camera {
    pub fn get_transform(&self) -> Matrix4<f32> {
        Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)
    }

    pub fn build_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        let view_matrix = self.get_transform().invert().unwrap();
        let projection_matrix = cgmath::perspective(
            self.vertical_fov,
            aspect_ratio,
            self.near_plane,
            self.far_plane,
        );

        OPENGL_TO_WGPU_MATRIX * projection_matrix * view_matrix
    }

    pub fn uniform(&self, aspect_ratio: f32) -> CameraUniform {
        CameraUniform {
            view_projection: self.build_view_projection_matrix(aspect_ratio).into(),
            _padding: [0; 3],
            aspect_ratio,
        }
    }

    pub fn world_to_screen_point(&self, aspect_ratio: f32, position: Vector3<f32>) -> Vector3<f32> {
        let transformed = self.build_view_projection_matrix(aspect_ratio) * position.extend(1.0);
        let divided = transformed.xy() / transformed.w;
        vec3(
            (divided.x + 1.0) / 2.0,
            (1.0 - divided.y) / 2.0,
            transformed.z,
        )
    }
}
