use crate::constants::*;
use crate::render::pipeline;
use crate::scene::Scene;
use crate::controls;
use crate::utility::read;

use std::{
    thread::sleep,
    time::{Duration, Instant}
};
use winit::{
	application::ApplicationHandler,
	event::{WindowEvent, DeviceEvent, DeviceId, KeyEvent},
	event_loop::{ActiveEventLoop},
	window::{Window, WindowId, CursorGrabMode},
};

//This will mostly work with winit as an app handler
pub struct EventHandler {
	pub window: Option<Window>, //Winit window that gets rendered to
	pub vulkan_app: Option<pipeline::VulkanApp>, //VulkanApp
	pub scene: Scene, //The scene containing all the fun stuff

	pub control_queues: controls::ControlQueues,
}

impl EventHandler {
	//Initialize empty app handler
	pub fn init() -> EventHandler {
		//Hold all controls queues here
		let control_queues = controls::ControlQueues::init();

		EventHandler {
			window: None,
			vulkan_app: None,
			scene: Scene::init_scene(),

			control_queues
		}
	}

	//Game loop - called on redraw request in "window_event" fn
	fn game_loop(&mut self, event_loop: &ActiveEventLoop) {
		let vulkan_app = self.vulkan_app.as_ref().unwrap();
		let window = self.window.as_ref().unwrap();
		let scene = &mut self.scene;

		let control_queues = &mut self.control_queues;

		//Get the initial time
		let initial_time = Instant::now();

		//Execute all the controls that happened this frame, then clear the control queue
		control_queues.execute_controls(vulkan_app, window, scene, event_loop);
		self.control_queues.clear();

		//Update the scene
		scene.update();

		//Acquire a swapchain image, render to it, then present it from the swapchain
		vulkan_app.draw_frame(window, scene);

		//Target time for one frame
		let frame_time = Duration::from_secs_f32(1.0 / FPS);
		//Check the elapsed time
		let elapsed_time = initial_time.elapsed();

		//Right now, just sleep until the next frame-ish time
		//If loop was too long, just run the next one ASAP
		//This is a horrible way to do it, but works for now
		if elapsed_time < frame_time {
			let time_to_sleep = frame_time - elapsed_time;
			sleep(time_to_sleep);
		} else {
			println!("Skipped frame at {:?} fps", FPS);
		}


		//Request a redraw for next frame
		window.request_redraw();
	}

	//Will call this after a "resumed" event - set everything up
	fn setup(&mut self, event_loop: &ActiveEventLoop) {
		//Setup the window attributes
		let window_attributes = Window::default_attributes()
			.with_title(WINDOW_TITLE)
			.with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
			.with_resizable(true)
			.with_window_icon(Some(read::icon_asset()));

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
}

//Winit stuff - application handler
impl ApplicationHandler for EventHandler {
	//This event happens whenever the application is resumed (or when first ran)
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		//Call the setup function defined above
		self.setup(event_loop);
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
		match event {
			//Event when close is requested. The winit example does it fancier with just setting a bool, then checks that bool later
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				event_loop.exit();
			},

			WindowEvent::Destroyed => {
				event_loop.exit();
			},

			//Called when OS requests a redraw
			WindowEvent::RedrawRequested => {
				//This will request another redraw after drawing a frame
				self.game_loop(event_loop);
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
				//Also resize the camera
				if width > 0 && height > 0 {
					vulkan_app.recreate_swapchain(window);
					self.scene.camera.camera_window_resize(window);
				}
			}

			//Event when key is pressed (as long as it isn't repeated, like when you hold a letter in a word doc and it keeps typing)
			WindowEvent::KeyboardInput {event: KeyEvent{repeat: false, ..}, ..} => {
				self.control_queues.push_key(event);
			},

			//Event when mouse button is pressed
			WindowEvent::MouseInput {..} => {
				self.control_queues.push_mouse(event)
			},

			//Any other event does nothing
			_ => (),
		}
	}

	//Handle device event here - since mouse movement is the only one I care about, just use an if let
	fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
		if let DeviceEvent::MouseMotion {..} = event {
			self.control_queues.push_raw_mouse(event)
		}
	}
}