use ash::vk;
use std::ffi::CStr;

//Consts for window creation
pub const WINDOW_TITLE: &str = "Jarmungular Engine";
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;

//Desired presentation mode option. Check for this in "VulkanApp::choose_presentation_mode" fn
pub const DESIRED_PRESENTATION_MODE: vk::PresentModeKHR = vk::PresentModeKHR::MAILBOX;

//Can enable/disable validation layers for debug/release
pub const VALIDATION_ENABLED: bool = true;
pub const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

//Required instance and device extensions are here
pub const INSTANCE_EXTENSIONS: [&CStr; 2] = [
	vk::KHR_SURFACE_NAME,
	vk::KHR_WIN32_SURFACE_NAME, //WINDOWS ONLY for now
];
pub const DEVICE_EXTENSIONS: [&CStr; 1] = [vk::KHR_SWAPCHAIN_NAME];

//Mouse sensitivity - measured in degrees per mouse increment
pub const SENSITIVITY: f32 = 0.04;

//FPS - will make a setting later
pub const FPS: f32 = 240.0;