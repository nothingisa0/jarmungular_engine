//INCLUDING SHADER FILES IN THE EXE NOW - DON'T NEED TO READ THE FILES
//WILL NEED TO USE THIS MODULE FOR ANY CUSTOM FILE TYPES LATER THOUGH, AS A "PARSER"

// use std::path::Path;
// use std::fs::read;
use winit::window::{Icon};





// //Reads shader spirv code
// pub fn r_shader(shader_path_str: &str) -> Vec<u8> {
// 	//Check if it's a spv file (or at least checks that it ends with an "spv" extension)
// 	if !shader_path_str.ends_with("spv") {panic!("Shader must be in SPIRV format")};

// 	//Convert the shader path str to a Path
// 	let shader_path = Path::new(shader_path_str);

// 	//Get and return the shader file as a vec of bytes
// 	read(shader_path).expect("Unable to read shader file")
// }

//Returns the icon for the app. This is an include, so it'll compile into the exe.
pub fn icon_asset() -> Icon {
	let icon_bytes = include_bytes!("C:/Users/jagan/Documents/Code/jarmungular_engine/src/assets/icon.rgba").to_vec();
	Icon::from_rgba(icon_bytes, 32, 32).expect("Bad icon")
}