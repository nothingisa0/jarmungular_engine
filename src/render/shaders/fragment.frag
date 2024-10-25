#version 460
//FRAGMENT SHADER

//Get input from the vertex shader
//Name doesn't necessarily need to match, just indices
layout(location = 0) in vec3 fragColor;
//Output the color RBGa
layout(location = 0) out vec4 outColor;

void main() {
    //Automatically gets interpolated
    outColor = vec4(fragColor, 1.0);
}