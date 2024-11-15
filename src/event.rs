use crate::constants::*;
use crate::render::pipeline;
use crate::scene::Scene;
use crate::utility::read::{icon_asset};

use std::{
    thread::sleep,
    time::{Duration, Instant}
};
use winit::{
	application::ApplicationHandler,
	event::{WindowEvent, DeviceEvent, DeviceId, ElementState, MouseButton},
	event_loop::{ActiveEventLoop},
	window::{Window, WindowId, CursorGrabMode},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
};

//This will mostly work with winit as an app handler
pub struct EventHandler {
	window: Option<Window>, //Winit window that gets rendered to
	vulkan_app: Option<pipeline::VulkanApp>, //VulkanApp
	scene: Scene, //The scene containing all the fun stuff
}

impl EventHandler {
	//Initialize empty app handler
	pub fn init() -> EventHandler {
		EventHandler {
			window: None,
			vulkan_app: None,
			scene: Scene::init_scene(),
		}
	}

	//Game loop - called on redraw request in "window_event" fn
	fn game_loop(vulkan_app: &pipeline::VulkanApp, window: &Window, scene: &Scene) {
		//Get the initial time
		let initial_time = Instant::now();

		//Acquire a swapchain image, render to it, then present it from the swapchain
		vulkan_app.draw_frame(window, scene);

		//Right now, just sleep for a little bit
		sleep(Duration::from_millis(10));

		//Check the elapsed time
		let elapsed_time = initial_time.elapsed();

		//Request a redraw for next frame
		window.request_redraw();
	}

	//A big match statement for the controls, to be called on a key press event
	//Press/release are defined under "state"
	//Match things as a tuple of the key and its press/release state. Later, might also want to pass in something like a character state (grounded, jumpsquat, etc), idk
	//Not sure how this would handle something like a "sprint key." I think it would have to turn on/off a "sprint" player state on press/release, and the sprint state would change the behavior of other controls (eg walk -> run)
	//Some people store the key states in a hash set, but I don't think that's necessary in a game context
	fn keyboard_controls(&self, event_loop: &ActiveEventLoop, key: &Key, key_state: ElementState) {
		//Matching both the key and the state
		match (key.as_ref(), key_state) {
			//Test key r
			(Key::Character("r"), ElementState::Pressed) => {
				println!("r key pressed");
			},

			//Esc key. Again, the winit example does it fancier with just setting a bool, then checks that bool later
			(Key::Named(NamedKey::Escape), ElementState::Pressed) => {
				println!("The esc button was pressed; stopping");
				self.close_app(event_loop);
			},

			_ => (),
		}
	}

	//Mouse button press
	fn mouse_controls(&self, window: &Window, button: MouseButton) {
		//Matching the mouse button pressed
		match button {
			MouseButton::Right => {
				window.set_cursor_visible(true);
				window.set_cursor_grab(CursorGrabMode::None).expect("Failed to set cursor mode");
			}
			_ => ()
		};
	}





	//Need to do mouse movement separately as a "device event"
	//Raw mouse input stuff
	fn mouse_movement(&mut self, event: DeviceEvent) {
		let window = self.window.as_ref().unwrap();
		let camera = &mut self.scene.camera;

		//Mouse movement will move the camera
		if let DeviceEvent::MouseMotion{delta} = event {camera.rotate_view(delta.0 as f32, -delta.1 as f32)};
	}

	//What to do when closing the app
	//Have to wait 
	fn close_app(&self, event_loop: &ActiveEventLoop) {
		self.vulkan_app.as_ref().unwrap().wait_idle();
		event_loop.exit();
	}
}

//Winit stuff - application handler
impl ApplicationHandler for EventHandler {
	//This event happens whenever the application is resumed (or when first ran)
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		//Setup the window attributes for the "window" field of the vulkanapp struct
		let window_attributes = Window::default_attributes()
			.with_title(WINDOW_TITLE)
			.with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
			.with_resizable(true)
			.with_window_icon(Some(icon_asset()));

		let window = event_loop.create_window(window_attributes).expect("Failed to create window");
		
		//Set cursor to be hidden and locked within the window
		window.set_cursor_visible(false);
		window.set_cursor_grab(CursorGrabMode::Confined).expect("Failed to set cursor mode");

		//Then set up the vulkan app
		let vulkan_app = pipeline::VulkanApp::init_vulkan(&window);

		//Set vulkan app handler's fields now
		self.window = Some(window);
		self.vulkan_app = Some(vulkan_app);

	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
		match event {
			//Event when close is requested. The winit example does it fancier with just setting a bool, then checks that bool later
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				self.close_app(event_loop);
			},

			//Called when OS requests a redraw
			WindowEvent::RedrawRequested => {
				//Make new variables for references to the app/window (to call methods on, pass into funtions)
				let vulkan_app = self.vulkan_app.as_ref().unwrap();
				let window = self.window.as_ref().unwrap();

				//This will request a redraw after drawing a frame
				EventHandler::game_loop(vulkan_app, window, &self.scene);
			},

			//Event when key is pressed
			//Matches a "KeyboardInput" struct that has "event" field with "Keyevent" struct. This "Keyevent" struct has all the juicy info
			//Will ignore repeated key events (when a key is held down), as repeat is matched to "false" for this branch
			WindowEvent::KeyboardInput {event, ..} => {
				//Get the key WITHOUT any modifiers (like shift)
				let key = event.key_without_modifiers();
				let key_state = event.state;
				//As long as it's not a repeated key, go into the "controls" fn
				//This was done before in the match statement using "{event: KeyEvent {logical_key: key, state, repeat: false, .. }, ..}" but that broke the key_without_modifiers
				if !event.repeat {self.keyboard_controls(event_loop, &key, key_state);};
			},

			WindowEvent::MouseInput {button, ..} => {
				let window = self.window.as_ref().unwrap();
				self.mouse_controls(window, button);
			},

			//Called when window is resized
			WindowEvent::Resized(size) => {
				//Make new variables for references to the app/window (to call methods on, pass into funtions)
				let vulkan_app = self.vulkan_app.as_mut().unwrap();
				let window = self.window.as_ref().unwrap();
				
				//Make sure window width > 0 and height > 0. If not, we won't do any resizing/drawing
				let width = window.inner_size().width;
				let height = window.inner_size().height;
				
				//If width and height are both nonzero, recreate the swapchain and all the jazz that comes with it
				if width > 0 && height > 0 {
					vulkan_app.recreate_swapchain(window);
				}
			}

			//Any other event does nothing
			_ => (),
		}
	}

	//Handle mouse movement here
	fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
		self.mouse_movement(event)
	}
}