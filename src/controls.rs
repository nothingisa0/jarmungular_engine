use crate::render::pipeline;
use crate::scene::Scene;
use crate::utility::debug;

use std::f32::consts::PI;
use std::collections::HashSet;
use winit::{
	event::{WindowEvent, DeviceEvent, ElementState, MouseButton},
	event_loop::{ActiveEventLoop},
	window::{Window, CursorGrabMode},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
};
use glam::f32::{vec3, Mat3};

//Make a struct to hold all the events corresponding to different controls
pub struct ControlQueues {
	key_queue: Vec<WindowEvent>, //Will keep a tally of all the keyboard controls that need to be executed in a frame
	mouse_queue: Vec<WindowEvent>, //Same thing but for mouse
	raw_mouse_queue: Vec<DeviceEvent>, //Same thing but for raw mouse input type stuff

	held_keys: HashSet<Key>, //A set (no duplicate elements) of all the keys being held down
	held_mouse_buttons: HashSet<MouseButton>, //A set (no duplicate elements) of all the mouse buttons being held down

	knobs: Vec<debug::Knob>, //Holds all midi knob positions. Not really a queue, but this is the best place to put it.
}

impl ControlQueues {
	pub fn init() -> ControlQueues {
		ControlQueues {
			key_queue: vec![],
			mouse_queue: vec![],
			raw_mouse_queue: vec![],

			held_keys: HashSet::new(),
			held_mouse_buttons: HashSet::new(),

			knobs: vec![debug::Knob::init(); 8],
		}
	}

	pub fn clear(&mut self) {
		self.key_queue.clear();
		self.mouse_queue.clear();
		self.raw_mouse_queue.clear();
	}

	pub fn push_key(&mut self, event: WindowEvent) {
		self.key_queue.push(event);
	}

	pub fn push_mouse(&mut self, event: WindowEvent) {
		self.mouse_queue.push(event);
	}

	pub fn push_raw_mouse(&mut self, event: DeviceEvent) {
		self.raw_mouse_queue.push(event);
	}

	//Run through all the controls that happened in a frame, execute them
	//Key, mouse, and raw mouse queues are separate. The match statement is kinda running twice so all the fields of the enums can be extracted, but this way is much easier for readability
	pub fn execute_controls(&mut self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop) {
		self.keyboard_queue_execute(vulkan_app, window, scene, event_loop);
		self.mouse_queue_execute(vulkan_app, window, scene, event_loop);
		self.raw_mouse_queue_execute(vulkan_app, window, scene, event_loop);

		self.holds_execute(vulkan_app, window, scene, event_loop);
	}

	//Key press
	fn keyboard_queue_execute(&mut self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop) {
		//Loop through keyboard events
		for event in &self.key_queue {
			if let WindowEvent::KeyboardInput{device_id, event, is_synthetic} = event {
				let key = event.key_without_modifiers();
				let key_state = event.state;

				//Add or remove the key to the hashset of keys being held
				if key_state == ElementState::Pressed {
					self.held_keys.insert(key.clone()); //Key should be small enough for a clone to not matter
				} else if key_state == ElementState::Released {
					self.held_keys.remove(&key);
				}

				//Matching both the key and the state
				//Only include controls that should execute on press. Controls that execute while a key is held should be in "holds_execute" fn
				match (key.as_ref(), key_state) {
					//Esc key will close the program immediately
					(Key::Named(NamedKey::Escape), ElementState::Pressed) => {
						println!("The esc button was pressed; stopping");
						event_loop.exit();
					},

					//Debug with midi knobs
					(Key::Character("m"), ElementState::Pressed) => {
						//Setup value names being changed
						let to_adjust_names = [
								"move_accel",
								"kickoff_boost_factor",

								"max_move_velocity",

								"friction_deccel",
						];
						//Setup values to change
						let mut to_adjust_vec = vec![
							&mut scene.player.move_constants.move_accel,
							&mut scene.player.move_constants.kickoff_boost_factor,

							&mut scene.player.move_constants.max_move_velocity,

							&mut scene.player.move_constants.friction_deccel,

						];
						//Bounds for all the adjustable values
						let bounds_vec = vec![
							(0.1, 2.0), 
							(1.0, 2.5), 

							(0.5, 4.0), 

							(0.005, 0.1), 

						];

						let knob_id = debug::midi_debug_controls(&self.held_keys, &mut self.knobs, &mut to_adjust_vec, bounds_vec);
						println!("{:?} value adjusted to {:?}", to_adjust_names[knob_id], to_adjust_vec[knob_id]);
					}

					_ => (),
				}
			}
		}
	}

	//Mouse button press
	fn mouse_queue_execute(&mut self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &Scene, event_loop: &ActiveEventLoop) {
		//Loop through mouse events
		for event in &self.mouse_queue {
			if let WindowEvent::MouseInput{device_id, state, button} = event {

				//Add or remove the button to the hashset of buttons being held
				if *state == ElementState::Pressed {
					self.held_mouse_buttons.insert(*button);
				} else if *state == ElementState::Released {
					self.held_mouse_buttons.remove(button);
				}
				
				//Only include controls that should execute on press. Controls that execute while a key is held should be in "holds_execute" fn
				match (button, state) {
					(MouseButton::Right, ElementState::Pressed) => {
						window.set_cursor_visible(true);
						window.set_cursor_grab(CursorGrabMode::None).expect("Failed to set cursor mode");
					}

					(MouseButton::Left, ElementState::Pressed) => {
						window.set_cursor_visible(false);
						window.set_cursor_grab(CursorGrabMode::Confined).expect("Failed to set cursor mode");
					}

					_ => ()
				};
			}
		}
	}

	//Raw mouse input stuff
	fn raw_mouse_queue_execute(&self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop) {
		//Loop through raw mouse events
		for event in &self.raw_mouse_queue {
			if let DeviceEvent::MouseMotion{delta} = event {
				scene.camera.rotate_view_from_xy(delta.0 as f32, -delta.1 as f32);
			};
		}
	}

	//Execute anything that should repeated on hold (like wasd movement)
	//Uses the hashsets populated from the other execute fns
	fn holds_execute(&self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop) {
		self.move_controls_execute(vulkan_app, window, scene, event_loop);
	}

	//Pass in the hashset, will do all the wasd/movement checks and pass into the appropriate function
	fn move_controls_execute(&self, vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop) {
		let mut dir = vec3(0.0, 0.0, 0.0);

		//Forward direction
		if self.held_keys.contains(&Key::Character("w".into())) {
			let pos = scene.camera.get_pos();
			let forward = scene.camera.get_forward_dir();

			dir += vec3(forward.x, 0.0, forward.z);
		};

		//Left direction
		if self.held_keys.contains(&Key::Character("a".into())) {
			let pos = scene.camera.get_pos();
			let forward = scene.camera.get_forward_dir();
			let rotation_matrix = Mat3::from_rotation_y(PI / 2.0);
			let left = rotation_matrix * forward;

			dir += vec3(left.x, 0.0, left.z);
		};

		//Backward direction
		if self.held_keys.contains(&Key::Character("s".into())) {
			let pos = scene.camera.get_pos();
			let forward = scene.camera.get_forward_dir();
			let rotation_matrix = Mat3::from_rotation_y(PI);
			let backward = rotation_matrix * forward;

			dir += vec3(backward.x, 0.0, backward.z);
		};

		//Right direction
		if self.held_keys.contains(&Key::Character("d".into())) {
			let pos = scene.camera.get_pos();
			let forward = scene.camera.get_forward_dir();
			let rotation_matrix = Mat3::from_rotation_y(3.0 * PI / 2.0);
			let right = rotation_matrix * forward;

			dir += vec3(right.x, 0.0, right.z);
		};

		//If the direction is nonzero, move the dude
		//Direction will be normalized when passed into the "move_grounded" fn
		if dir.length_squared() > 0.0 {
			scene.player.move_grounded(dir);
		}
	}
}