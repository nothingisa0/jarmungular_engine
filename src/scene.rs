pub mod camera;
pub mod entities;

use crate::scene::camera::Camera;
use crate::scene::entities::Player;
use crate::render::Vertex;

use glam::f32::{vec3, vec4};

//Make the vertices for a test triangle
pub const TEST_TRIANGLE_VERTICES: [Vertex; 9] = [
	Vertex {pos: vec4(  20.0,100.0,  0.0,  1.0), color: vec3(1.0, 0.0, 0.0)},
	Vertex {pos: vec4(  20.0, -5.0, -5.0,  1.0), color: vec3(0.0, 1.0, 0.0)},
	Vertex {pos: vec4(  20.0, -5.0,  13.0,  1.0), color: vec3(0.0, 0.0, 1.0)},

	Vertex {pos: vec4(  20.0,100.0,  0.0,  1.0), color: vec3(1.0, 0.0, 0.0)},
	Vertex {pos: vec4(  20.0, -5.0,  13.0, 1.0), color: vec3(0.0, 1.0, 0.0)},
	Vertex {pos: vec4(  20.0, -5.0, -5.0,  1.0), color: vec3(0.0, 0.0, 1.0)},

	Vertex {pos: vec4(  20.0, -5.0,  10.0,  1.0), color: vec3(1.0, 1.0, 1.0)},
	Vertex {pos: vec4(  20.0,-10.0, -5.0,  1.0), color: vec3(0.0, 0.0, 0.0)},
	Vertex {pos: vec4(  20.0,-10.0,  5.0,  1.0), color: vec3(0.0, 0.0, 0.0)},
];

//Scene with all the stuff in it
pub struct Scene {
	pub camera: Camera,
	pub player: Player,
}

impl Scene {
	//Right now, doesn't really do much
	pub fn init_scene() -> Scene {
		//Create the camera by passing in the camera pos and target
		let camera = Camera::new(vec3(0.0, 0.0, 10.0), vec3(20.0, 100.0, 0.0));
		//Create the player by passing in pos
		let player = Player::new(vec3(0.0, 0.0, 10.0));
		
		//Return the initialized scene
		Scene {
			camera,
			player,
		}
	}

	//Get the result from the camera's "render_matrix" fn
	pub fn get_render_matrix_bytes(&self) -> [u8; 64] {
		let render_matrix = self.camera.get_render_matrix();

		//Transmute is a little evil but I don't care
		unsafe { std::mem::transmute::<glam::f32::Mat4, [u8; 64]>(render_matrix) }
	}

	//Update scene physics. This will be called per timestep (with some timestep/frame jank involved)
	pub fn update(&mut self) {
		//Update the player
		self.player.update();
		//Glue the camera to the player
		let player_pos = self.player.get_pos();
		self.camera.set_pos(player_pos.x, player_pos.y, player_pos.z);
	}
}