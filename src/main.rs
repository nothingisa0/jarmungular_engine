//TO DO: fix physical device suitability score so it (hopefully) works on luke's computer and doesn't do the hookapp thing.
//TO DO: device events for mouse movement
//TO DO: get rid of multiple command buffers, record to command buffer at runtime. This should be cheaper when only one frame is in flight at a time
	//Frames in flight are only there to give CPU something to do while GPU chugs away, but they increase lag by letting the CPU game physics go farther ahead than the rendering
//TO DO: If recording command buffers is done as above, can also try to take pipeline creation + command buffer creation out of "resize swapchain"

//CONSIDER: not rendering directly to swapchain - instead rendering to a separate image and then copy to swapchain (separating rending and presentation). Will need for mirrors and postprocessing. Use sascha example.
	//Maybe something like: a render pass for all the mirrors in the scene, depth/stencil prepass (for mirror stencil, might not need, depth prepass may help forward renderer), postprocessing pass, pass that renders to swapchain
//CONSIDER: Might have to handle minimized windows better. It pretty much pauses presentation right now, which isn't the winit recommended solution.



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
