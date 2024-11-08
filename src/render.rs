pub mod handler;
pub mod pipeline;
pub mod constants;

use ash::vk;
use glam::f32::{vec3, Vec3, vec4, Vec4};

pub struct Vertex {
	pos: Vec4,
	color: Vec3,
}

impl Vertex {
	//Initializes a new vertex
	pub fn new(pos: Vec4, color: Vec3) -> Vertex {
		Vertex {
			pos,
			color,
		}
	}

	//Get vertex binding descriptions for vulkan
	//Specifies how an array of vertices will be passed to the vertex shader
	pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
		//Only need one of these, since the vertex data is all in one array
		[vk::VertexInputBindingDescription {
			binding: 0, //Index of this binding in the array of bindings
			stride: core::mem::size_of::<Vertex>() as u32, //Bytes from one entry to the next (in this case, size of the vertex struct)
			input_rate: vk::VertexInputRate::VERTEX, //Move to the next data entry after each vertex or instance - "vk::VertexInputRate::VERTEX" would be required for instanced rendering
		}]
	}

	//Set vertex attributes for vulkan
	//Specifies how to extract vertex attributes (position, color, etc) originating from a binding description
	pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
		[
			//Vertex attribute description
			vk::VertexInputAttributeDescription {
				location: 0, //Matches the location specified in the vertex shader (ex: layout(location = 0) in vec4 inPosition)
				binding: 0, //Index of this description in the array of bindings
				format: vk::Format::R32G32B32A32_SFLOAT, //Use a color format that corresponds to the component number/type of the vector in the shader. Here, for pos, vec4 would be VK_FORMAT_R32G32B32A32_SFLOAT
				offset: core::mem::offset_of!(Vertex, pos) as u32, //Byte offset relative to the start of a entry, rust has a nice macro for that
			},
			//Color attribute description
			vk::VertexInputAttributeDescription {
				location: 1,
				binding: 0,
				format: vk::Format::R32G32B32_SFLOAT, //Here, for color, vec4 would be VK_FORMAT_R32G32B32_SFLOAT
				offset: core::mem::offset_of!(Vertex, color) as u32,
			},
		]
	}
}



//Make the vertices for a test triangle
pub const TEST_TRIANGLE_VERTICES: [Vertex; 3] = [
	Vertex {pos: vec4( 0.0, -0.5,  0.0,  1.0), color: vec3(1.0, 0.0, 0.0)},
	Vertex {pos: vec4(-0.5,  0.5,  0.0,  1.0), color: vec3(0.0, 1.0, 0.0)},
	Vertex {pos: vec4( 0.5,  0.5,  0.0,  1.0), color: vec3(0.0, 0.0, 1.0)},
];