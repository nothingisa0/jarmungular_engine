use winit::window::{Icon};

//This module is meant to include binary files into the exe at compile time
//If size ever becomes a problem, might have to:
//	Compress the bin
//	Include it in the exe at the top of each "read" function in this module
//	Decompress the bin at runtime
//Could be a fun little project to implement, but I don't see myself really needing it

//Reads fragment shader spirv code
pub fn fragment_shader() -> Vec<u8> {
	include_bytes!("../render/shaders/fragment.spv").to_vec()
}

//Reads vertex shader spirv code
pub fn vertex_shader() -> Vec<u8> {
	include_bytes!("../render/shaders/vertex.spv").to_vec()
}

//Returns the icon for the app from an rgba file
pub fn icon_asset() -> Icon {
	let icon_bytes = include_bytes!("../assets/icon.rgba").to_vec();
	Icon::from_rgba(icon_bytes, 32, 32).expect("Bad icon")
}