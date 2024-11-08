#version 460
//VERTEX SHADER
//Outputs the clip coordinates (xyzw, not yet normalized) as gl_Position (predefined output)

//Define inputs
//layout(location = 0) specifies framebuffer index
//Some variables (dvec3, for example) will take up multiple slots so the next index must be higher, be careful with that
layout(location = 0) in vec4 inPosition;
layout(location = 1) in vec3 inColor;

//Define outputs
//gl_Position is a predefined output
//layout(location = 0) specifies framebuffer index
layout(location = 0) out vec3 fragColor;

//Main function is called for every vertex
void main() {
	//Sets vertex position
	gl_Position = inPosition;
	//Sets vertex color (this just is an output to pass to the fragment shader)
	fragColor = inColor;
}