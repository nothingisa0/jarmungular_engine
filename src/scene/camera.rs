use crate::render::constants::{WINDOW_WIDTH, WINDOW_HEIGHT, SENSITIVITY};

use glam::f32::{vec3, Vec3, Mat3, Mat4};
use winit::window::Window;

pub struct Camera {
	pos: Vec3, //The position of the camera
	target: Vec3, //Target the camera is pointing to as a point in world space (x, y, z)

	fov_y_radians: f32, //Radians from top to bottom
	aspect_ratio: f32, //Width over height
	z_near: f32, //Near clipping plane
	z_far: f32, //Far clipping plane

	pub view_matrix: Mat4, //View matrix that will be calculated
	pub projection_matrix: Mat4, //Projection matrix that will be calculated
	pub render_matrix: Mat4, //Combination of the view and projection matrices that will be calculated
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

			//Have all the matrices in as identity - will need to populate them after
			view_matrix: Mat4::IDENTITY,
			projection_matrix: Mat4::IDENTITY,
			render_matrix: Mat4::IDENTITY,
		}
	}

	//Calculate all the matrices
	//This will need to be called if any of the camera's fields are adjusted manually, rather than through Camera's methods
	pub fn calc_matrices(&mut self) {
		self.view_matrix = self.calc_view_matrix();
		self.projection_matrix = self.calc_projection_matrix();
		self.render_matrix = self.calc_render_matrix();
	}

	//Returns the view matrix
	fn calc_view_matrix(&self) -> Mat4 {
		let pos = self.pos;
		let target = self.target;
		let up = vec3(0.0, -1.0, 0.0);

		Mat4::look_at_rh(pos, target, up)
	}

	//Returns the perspective projection matrix
	//Maps to depth range [0, 1]
	fn calc_projection_matrix(&self) -> Mat4 {
		Mat4::perspective_rh(self.fov_y_radians, self.aspect_ratio, self.z_near, self.z_far)
	}
	
	//Uses the camera to do determine the render matrix for the scene
	fn calc_render_matrix(&self) -> Mat4 {
		let view_matrix = self.view_matrix;
		let projection_matrix = self.projection_matrix;
		projection_matrix * view_matrix
	}

	//Will rotate view when passed an x and y. Will use this for mouse movement
	pub fn rotate_view(&mut self, x: f32, y: f32) {
		use std::f32::consts::PI;
		
		let x_angle = y * SENSITIVITY * PI / 180.0; //y movement rotates about x (degrees, where sensitivity is in degrees per mouse increment)
		let y_angle = x * SENSITIVITY * PI / 180.0; //x movement rotates about y (degrees, where sensitivity is in degrees per mouse increment)
		
		let x_rotation_matrix = Mat3::from_rotation_x(x_angle);
		let y_rotation_matrix = Mat3::from_rotation_y(y_angle);

		//The direction the camera faces
		let mut dir = self.target - self.pos;
		dir = y_rotation_matrix * x_rotation_matrix * dir;

		self.target = dir + self.pos;

		self.calc_matrices();
	}

	//Modifies the camera's aspect ratio when window is resized
	pub fn camera_window_resize(self, window: &Window) {
		todo!();
	}
}