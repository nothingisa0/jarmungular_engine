use ash::vk;
use std::ffi::CStr;

//Consts for window creation
pub const WINDOW_TITLE: &str = "peepee poopoo";
pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 600;

//Can enable/disable validation layers for debug/release
pub const VALIDATION_ENABLED: bool = true;
pub const VALIDATION_LAYERS: [&str; 1] = ["VK_LAYER_KHRONOS_validation"];

//Required instance and device extensions are here
pub const INSTANCE_EXTENSIONS: [&CStr; 2] = [
        vk::KHR_SURFACE_NAME,
        vk::KHR_WIN32_SURFACE_NAME, //WINDOWS ONLY for now
    ];
pub const DEVICE_EXTENSIONS: [&CStr; 1] = [vk::KHR_SWAPCHAIN_NAME];