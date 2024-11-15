pub mod camera;

use crate::scene::camera::Camera;
use crate::render::Vertex;

use glam::f32::{vec3, vec4};

//Make the vertices for a test triangle
pub const TEST_TRIANGLE_VERTICES: [Vertex; 3] = [
	Vertex {pos: vec4(  0.0,100.0,  0.0,  1.0), color: vec3(1.0, 0.0, 0.0)},
	Vertex {pos: vec4(  5.0, -5.0,  0.0,  1.0), color: vec3(0.0, 1.0, 0.0)},
	Vertex {pos: vec4( -5.0, -5.0,  0.0,  1.0), color: vec3(0.0, 0.0, 1.0)},
];

//Scene with all the stuff in it
pub struct Scene {
	pub camera: Camera,
}

impl Scene {
	//Right now, doesn't really do much
	pub fn init_scene() -> Scene {
		//Create the camera by passing in the camera pos and target
		let camera = Camera::new(vec3(0.0, 0.0, 20.0), vec3(0.0, 100.0, 0.0));
		
		//Return the initialized scene
		Scene {
			camera,
		}
	}

	//Get the result from the camera's "render_matrix" fn
	pub fn render_matrix_bytes(&self) -> [u8; 64] {
		let render_matrix = self.camera.get_render_matrix();

		//This method will do it safely, but it takes longer per frame
			// let mut render_matrix_slice: [f32; 16] = [0.0; 16];
			
			// render_matrix.write_cols_to_slice(&mut render_matrix_slice);
			
			// let render_matrix_bytes: [u8;64] = render_matrix_slice.iter()
			// 	.flat_map(|slice| slice.to_le_bytes()
			// 	.into_iter())
			// 	.collect::<Vec<u8>>()
			// 	.try_into().expect("Failed to convert render matrix");

			// render_matrix_bytes

		//Transmute is mega-evil and mega-unsafe, but it works faster so I'm not gonna let neckbeards on reddit dictate my life
		//(Don't tell anyone it's literally only like .1ms per frame)
		unsafe { std::mem::transmute::<glam::f32::Mat4, [u8; 64]>(render_matrix) }
	}
}