use crate::render::pipeline;
use crate::scene::Scene;

use winit::{
	event::{WindowEvent, DeviceEvent, ElementState, MouseButton},
	event_loop::{ActiveEventLoop},
	window::{Window, CursorGrabMode},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
};

//Make a struct to hold all the events corresponding to different controls
pub struct ControlQueues {
	key_queue: Vec<WindowEvent>, //Will keep a tally of all the keyboard controls that need to be executed in a frame
	mouse_queue: Vec<WindowEvent>, //Same thing but for mouse
	raw_mouse_queue: Vec<DeviceEvent>, //Same thing but for raw mouse input type stuff
}

impl ControlQueues {
	pub fn init() -> ControlQueues {
		ControlQueues {
			key_queue: vec![],
			mouse_queue: vec![],
			raw_mouse_queue: vec![],
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
}

//Run through all the controls that happened in a frame, execute them
//Key, mouse, and raw mouse queues are separate so the match statement doesn't have to run twice
pub fn execute_controls(vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop, control_queues: &ControlQueues) {
	let key_queue = &control_queues.key_queue;
	let mouse_queue = &control_queues.mouse_queue;
	let raw_mouse_queue = &control_queues.raw_mouse_queue;

	for event in key_queue {
		keyboard_controls(vulkan_app, window, scene, event_loop, &event);
	}

	for event in mouse_queue {
		mouse_controls(vulkan_app, window, scene, event_loop, &event);
	}

	for event in raw_mouse_queue {
		raw_mouse_controls(vulkan_app, window, scene, event_loop, &event);
	}
}

//Key press
fn keyboard_controls(vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &Scene, event_loop: &ActiveEventLoop, event: &WindowEvent) {
	if let WindowEvent::KeyboardInput{device_id, event, is_synthetic} = event {
		let key = event.key_without_modifiers();
		let key_state = event.state;

		//Matching both the key and the state
		match (key.as_ref(), key_state) {
			//Test key r
			(Key::Character("r"), ElementState::Pressed) => {
				println!("r key pressed");
			},

			//Esc key. Again, the winit example does it fancier with just setting a bool, then checks that bool later
			(Key::Named(NamedKey::Escape), ElementState::Pressed) => {
				println!("The esc button was pressed; stopping");
				event_loop.exit();
			},

			_ => (),
		}
	}
}

//Mouse button press
fn mouse_controls(vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &Scene, event_loop: &ActiveEventLoop, event: &WindowEvent) {
	if let WindowEvent::MouseInput{device_id, state, button} = event {
		match button {
			MouseButton::Right => {
				window.set_cursor_visible(true);
				window.set_cursor_grab(CursorGrabMode::None).expect("Failed to set cursor mode");
			}
			_ => ()
		};
	}
}

//Raw mouse input stuff
fn raw_mouse_controls(vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &mut Scene, event_loop: &ActiveEventLoop, event: &DeviceEvent) {
	if let DeviceEvent::MouseMotion{delta} = event {
		scene.camera.rotate_view_from_xy(delta.0 as f32, -delta.1 as f32)
	};
}










// //A big match statement for the controls, to be called on a key press event
// //Press/release are defined under "state"
// //Match things as a tuple of the key and its press/release state. Later, might also want to pass in something like a character state (grounded, jumpsquat, etc), idk
// //Not sure how this would handle something like a "sprint key." I think it would have to turn on/off a "sprint" player state on press/release, and the sprint state would change the behavior of other controls (eg walk -> run)
// //Some people store the key states in a hash set, but I don't think that's necessary in a game context
// fn keyboard_controls(event_handler: &EventHandler, event_loop: &ActiveEventLoop, event: WindowEvent) {
// 	//Get the key WITHOUT any modifiers (like shift)
// 	let key = event.key_without_modifiers();
// 	let key_state = event.state;

// 	//Matching both the key and the state
// 	match (key.as_ref(), key_state) {
// 		//Test key r
// 		(Key::Character("r"), ElementState::Pressed) => {
// 			println!("r key pressed");
// 		},

// 		//Esc key. Again, the winit example does it fancier with just setting a bool, then checks that bool later
// 		(Key::Named(NamedKey::Escape), ElementState::Pressed) => {
// 			println!("The esc button was pressed; stopping");
// 			self.close_app(event_loop);
// 		},

// 		_ => (),
// 	}
// }

// //Mouse button press
// fn mouse_controls(event_handler: &EventHandler, event_loop: &ActiveEventLoop, event: WindowEvent) {
// 	//Matching the mouse button pressed
// 	match button {
// 		MouseButton::Right => {
// 			window.set_cursor_visible(true);
// 			window.set_cursor_grab(CursorGrabMode::None).expect("Failed to set cursor mode");
// 		}
// 		_ => ()
// 	};
// }

// //Need to do mouse movement separately as a "device event"
// //Raw mouse input stuff
// fn mouse_movement(event_handler: &EventHandler, event_loop: &ActiveEventLoop, event: WindowEvent) {
// 	let window = window.as_ref().unwrap();
// 	let camera = scene.camera;

// 	//Mouse movement will move the camera
// 	if let DeviceEvent::MouseMotion{delta} = event {camera.rotate_view_from_xy(delta.0 as f32, -delta.1 as f32)};
// }