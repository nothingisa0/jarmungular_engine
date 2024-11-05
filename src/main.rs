//TO DO: fix physical device suitability score so it (hopefully) works on luke's computer and doesn't do the hookapp thing.
//TO DO: A variety of resolutions. Probably will need to make the pipeline viewports/scissors dynamic.
//TO DO: device events for mouse movement

//CONSIDER: not rendering directly to swapchain - instead rendering to a separate image and then copy to swapchain (separating rending and presentation).
//CONSIDER: multiple frames in flight for FIFO/MAILBOX modes


//#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![windows_subsystem = "windows"] //This will disable the terminal popping up when the app is run

mod render;
mod utility;
use render::handler::VulkanAppHandler;
use winit::event_loop::{EventLoop, ControlFlow};


//Setup the winit event loop and start the app
fn main() {
	let event_loop = EventLoop::new().expect("Event loop creation failed");
	event_loop.set_control_flow(ControlFlow::Poll);
	let mut vulkan_app_handler = VulkanAppHandler::init();
	event_loop.run_app(&mut vulkan_app_handler).expect("Failed to run app");
}
