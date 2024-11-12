use crate::render::constants::{WINDOW_WIDTH, WINDOW_HEIGHT};

use glam::f32::{vec3, Vec3, Mat4};
use winit::window::Window;

pub struct Camera {
	pos: Vec3, //The position of the camera
	target: Vec3, //Where the camera is pointing

	fov_y_radians: f32, //Radians from top to bottom
	aspect_ratio: f32, //Width over height
	z_near: f32, //Near clipping plane
	z_far: f32, //Far clipping plane

}

impl Camera {
	//Make a camera, initialize it with default values
	pub fn new(pos: Vec3, target: Vec3) -> Camera {
		Camera {
			pos,
			target,

			fov_y_radians: 0.7, //About 40 degrees
			aspect_ratio: (WINDOW_WIDTH / WINDOW_HEIGHT) as f32, //Constant aspect ratio on initialization
			z_near: 1.0, //Will probably need to play around with
			z_far: 10000.0,
		}
	}

	//Returns the view matrix
	pub fn view_matrix(&self) -> Mat4 {
		let eye = self.pos;
		let center = self.target;
		let up = vec3(0.0, -1.0, 0.0);

		Mat4::look_at_rh(eye, center, up)
	}

	//Returns the perspective projection matrix
	//Maps to depth range [0, 1]
	pub fn projection_matrix(&self) -> Mat4 {
		Mat4::perspective_rh(self.fov_y_radians, self.aspect_ratio, self.z_near, self.z_far)
	}
	
	//Uses the camera to do determine the render matrix for the scene
	pub fn render_matrix(&self) -> Mat4 {
		let view_matrix = self.view_matrix();
		let projection_matrix = self.projection_matrix();
		projection_matrix * view_matrix
	}

	//Will rotate view when passed an x and y. Will use this for mouse movement
	pub fn rotate_view(x: f32, y: f32) {

	}

	//Modifies the camera's aspect ratio when window is resized
	pub fn resize_for_window(self, window: &Window) {

	}
}