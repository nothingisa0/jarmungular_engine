#version 460
//VERTEX SHADER
//Outputs clip coordinates (xyzw, not yet normalize) as gl_Position (predefined output)

//Define outputs
//layout(location = 0) specifies framebuffer index
layout(location = 0) out vec3 fragColor;


//This is just going to give positions of the vertex in clip space at the moment. In the future, should be able to take world space and transform to NDC
vec2 positions[3] = vec2[](
	vec2(0.0, 0.5),
	vec2(0.5, 0.5),
	vec2(-0.5, -0.5)
);

//Predefined color array as well
vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

//Main function is called for every vertex
void main() {
	//Sets vertex position to be (x, y, 0, 1)
	gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
	//Sets vertex color (this just is an output to pass to the fragment shader)
	fragColor = colors[gl_VertexIndex];
}