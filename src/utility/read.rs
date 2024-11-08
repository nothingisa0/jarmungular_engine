use winit::window::{Icon};




//Reads shader spirv code
//Includes as bytes into the exe at compile time
pub fn fragment_shader() -> Vec<u8> {
	include_bytes!("C:/Users/jagan/Documents/Code/jarmungular_engine/src/render/shaders/fragment.spv").to_vec()
}
pub fn vertex_shader() -> Vec<u8> {
	include_bytes!("C:/Users/jagan/Documents/Code/jarmungular_engine/src/render/shaders/vertex.spv").to_vec()
}

//Returns the icon for the app. This is an include, so it'll compile into the exe.
pub fn icon_asset() -> Icon {
	let icon_bytes = include_bytes!("C:/Users/jagan/Documents/Code/jarmungular_engine/src/assets/icon.rgba").to_vec();
	Icon::from_rgba(icon_bytes, 32, 32).expect("Bad icon")
}