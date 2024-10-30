use std::path::Path;
use std::fs::read;

//Reads shader spirv code
pub fn r_shader(shader_path_str: &str) -> Vec<u8> {
	//Convert the shader path str to a Path
	let shader_path = Path::new(shader_path_str);

	//Get and return the shader file as a vec of bytes
	let spv_file = read(shader_path).expect("Unable to read shader file");
	spv_file
}