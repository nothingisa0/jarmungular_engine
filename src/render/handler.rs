use crate::render::constants::*;
use crate::render::pipeline;

use winit::{
	application::ApplicationHandler,
	event::{WindowEvent, ElementState},
	event_loop::{ActiveEventLoop},
	window::{Window, WindowId},
	keyboard::{Key, NamedKey},
	platform::modifier_supplement::KeyEventExtModifierSupplement,
};

//This will mostly work with winit as an app handler
pub struct VulkanAppHandler {
	window: Option<Window>, //Winit window that gets rendered to
	vulkan_app: Option<pipeline::VulkanApp> //VulkanApp
}

impl VulkanAppHandler {
	//Initialize empty app handler
	pub fn init() -> VulkanAppHandler {
		VulkanAppHandler {
			window: None,
			vulkan_app: None
		}
	}

	//A big match statement for the controls, to be called on a key press event
	//Press/release are defined under "state"
	//Match things as a tuple of the key and its press/release state. Later, might also want to pass in something like a character state (grounded, jumpsquat, etc), idk
	//Not sure how this would handle something like a "sprint key." I think it would have to turn on/off a "sprint" player state on press/release, and the sprint state would change the behavior of other controls (eg walk -> run)
	//Some people store the key states in a hash set, but I don't think that's necessary in a game context
	fn controls(event_loop: &ActiveEventLoop, key: &Key, key_state: ElementState) {
		//Matching both the key
		match (key.as_ref(), key_state) {
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

//Winit stuff - application handler
impl ApplicationHandler for VulkanAppHandler {
	//This event happens whenever the application is resumed (or when first ran)
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		//Setup the window attributes for the "window" field of the vulkanapp struct
		let window_attributes = Window::default_attributes()
			.with_title(WINDOW_TITLE)
			.with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
			.with_resizable(false);
		let window = event_loop.create_window(window_attributes).expect("Failed to create window");

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
				event_loop.exit();
			},

			//Event when key is pressed
			//Matches a "KeyboardInput" struct that has "event" field with "Keyevent" struct. This "Keyevent" struct has all the juicy info
			//Will ignore repeated key events (when a key is held down), as repeat is matched to "false" for this branch
			WindowEvent::KeyboardInput {event, ..} => {
				//Get the key WITHOUT any modifiers (like shift)
				let key = event.key_without_modifiers();
				let key_state = event.state;
				//As long as it's not a repeated key, go into the controls
				//This was done before in the match statement using "{event: KeyEvent {logical_key: key, state, repeat: false, .. }, ..}" but that broke with key_without_modifiers
				if !event.repeat {VulkanAppHandler::controls(event_loop, &key, key_state);};
			},

			//Called when OS requests a redraw
			WindowEvent::RedrawRequested => {
				self.vulkan_app.as_mut().unwrap().draw_frame();
				self.vulkan_app.as_mut().unwrap().wait_idle();
			},

			//Any other event does nothing
			_ => (),
		}
	}
}