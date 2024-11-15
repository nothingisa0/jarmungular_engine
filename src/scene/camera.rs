use crate::constants::{WINDOW_WIDTH, WINDOW_HEIGHT, SENSITIVITY};

use std::f32::consts::PI;
use glam::f32::{vec3, Vec3, Mat4};
use winit::window::Window;

pub struct Camera {
	pos: Vec3, //The position of the camera
	dir: Vec3, //Direction the camera is pointing towards as a normalized vector in world space - spherical coordinates (pitch about x, yaw about y, roll about z)

	fov_y_radians: f32, //Radians from top to bottom
	aspect_ratio: f32, //Width over height
	z_near: f32, //Near clipping plane
	z_far: f32, //Far clipping plane

	view_matrix: Mat4, //View matrix that will be calculated
	projection_matrix: Mat4, //Projection matrix that will be calculated
	render_matrix: Mat4, //Combination of the view and projection matrices that will be calculated
}

impl Camera {
	//Make a camera, initialize it with default values, then calculate the required matrices
	//Takes in a position and a target to be looking at (point in world space)
	pub fn new(pos: Vec3, target: Vec3) -> Camera {
		let dir_xyz = (target - pos).normalize_or(vec3(1.0, 0.0, 0.0));
		
		let pitch = (dir_xyz.y).asin();
		let yaw = dir_xyz.x.atan2(dir_xyz.y);
		let roll = 0.0;
		
		let dir = vec3(pitch, yaw, roll);

		//Make a camera
		let mut init_camera = Camera {
			pos,
			dir,

			fov_y_radians: 0.7, //About 40 degrees
			aspect_ratio: (WINDOW_WIDTH / WINDOW_HEIGHT) as f32, //Constant aspect ratio on initialization
			z_near: 1.0, //Will probably need to play around with
			z_far: 10000.0,

			//Have all the matrices in as identity - will need to populate them after
			view_matrix: Mat4::IDENTITY,
			projection_matrix: Mat4::IDENTITY,
			render_matrix: Mat4::IDENTITY,
		};

		//Calculate everything, return the camera
		init_camera.calc_matrices();
		init_camera
	}

	//Clamp pitch/yaw/roll and then calculate all the matrices
	//This will need to be called if any of the camera's fields are adjusted manually, rather than through Camera's methods
	fn calc_matrices(&mut self) {
		//Clamp pitch - keep it between -90 and 90 so neck won't break
		if self.dir.x < -PI / 2.0 {
			self.dir.x = -PI / 2.0;
		}
		if self.dir.x >= PI / 2.0 {
			self.dir.x = PI / 2.0;
		}

		//Clamp yaw, keeping remainder  - keep between -180 to 180 (full circle)
		if self.dir.y < -PI {
			self.dir.y += 2.0 * PI;
		}
		if self.dir.y > PI {
			self.dir.y -= 2.0 * PI;
		}

		//Clamp roll  - keep it between -90 and 90 so neck won't break
		if self.dir.z < -PI / 2.0 {
			self.dir.z = -PI / 2.0;
		}
		if self.dir.z >= PI / 2.0 {
			self.dir.z = PI / 2.0;
		}

		//Calculate matrices
		self.view_matrix = self.calc_view_matrix();
		self.projection_matrix = self.calc_projection_matrix();
		self.render_matrix = self.calc_render_matrix();
	}

	//Returns the view matrix
	fn calc_view_matrix(&self) -> Mat4 {
		let pos = self.pos;
		let pitch = self.dir.x;
		let yaw = self.dir.y;
		let roll = self.dir.z;

		//First need to get the y coordinate flipped to go from world space (rh, y is up) to vulkan's NDC (right hand, y is down)
		//Then need to multiply the position by the pitch/yaw/roll of the camera - mutiply by yaw first to gimbalize that axis
		//Then need to shift to account for camera position
		Mat4::from_rotation_z(roll) * Mat4::from_rotation_x(pitch) * Mat4::from_rotation_y(yaw) *  Mat4::from_translation(-pos) * Mat4::from_rotation_z(PI)
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

	//Rotates the camera given a pitch, roll, yaw to rotate by
	fn rotate_view(&mut self, pitch_adj: f32, yaw_adj: f32, roll_adj: f32) {
		let pitch = self.dir.x + pitch_adj;
		let yaw = self.dir.y + yaw_adj;
		let roll = self.dir.z + roll_adj;

		self.dir = vec3(pitch, yaw, roll);
		//Clamping pitch/yaw/roll will happen during matrix calculation
		self.calc_matrices();
	}

	//Make sure all matrices are updated, then return the render matrix
	//This WON'T calculate the render matrix first. Calculation should be done at the end of any functions that may mutate the camera
	pub fn get_render_matrix(&self) -> Mat4 {
		self.render_matrix
	}

	//Will rotate view when passed an x and y. Will use this for mouse movement
	pub fn rotate_view_from_xy(&mut self, x: f32, y: f32) {
		let pitch_adj = y * SENSITIVITY * PI / 180.0; //y mouse movement rotates about x axis (degrees, where sensitivity is in degrees per mouse increment)
		let yaw_adj = x * SENSITIVITY * PI / 180.0; //x mouse movement rotates about y axis (degrees, where sensitivity is in degrees per mouse increment)

		self.rotate_view(pitch_adj, yaw_adj, 0.0);
	}

	//Modifies the camera's aspect ratio when window is resized
	pub fn camera_window_resize(self, window: &Window) {
		todo!();
	}
}