use crate::render::constants::*;
use crate::utility::read::r_shader;

use ash::{vk, khr, Entry};
use winit::{
	window::{Window},
	raw_window_handle::{HasWindowHandle, RawWindowHandle},
};
use std::ffi::{CString, CStr};
use std::ptr;

//Extensions, can add more here if need be (these are instance extensions, not device extensions)
//vkEnumerateInstanceExtensionProperties checks which extensions are available in the vulkan implementation (on my computer)
//Can add more extensions here
//This isn't a const because it may be different on different devices
//Returns the extension names to pass to instance creation
fn required_extension_names(entry: &ash::Entry) -> Vec<*const i8> {
	//Check for available instance extensions and convert to a vec of &CStr
	let available_extensions: Vec<vk::ExtensionProperties> = unsafe {entry.enumerate_instance_extension_properties(None).expect("Failed to enumerate instance extensions")};
	let available_extension_names :Vec<&CStr> = available_extensions
		.iter()
		.map(|extension_properties| extension_properties.extension_name_as_c_str().unwrap())
		.collect();

	//Check the listed extensions against the instance, panic if one isn't available. Then returns the vec.
	//Exclamation point is just a "not"
	for extension_to_check in &INSTANCE_EXTENSIONS {
		if !available_extension_names.contains(extension_to_check) {
			panic!("Instance extension not available")
		}
	}

	INSTANCE_EXTENSIONS
		.iter()
		.map(|extension_properties_cstr| extension_properties_cstr.as_ptr())
		.collect()
}

//Validation Layers
//Can add more validation layers here
struct ValidationInfo<'a> { //I'm doing mine in a func, not as a const, so lifetime is <'a> instead of static - I think this just means that rust will deallocate the &str when the struct is deallocated
	is_enabled: bool,
	validation_layers: [&'a str; VALIDATION_LAYERS.len()]
}

//vkEnumerateInstanceLayerProperties checks which layers are available in the vulkan implementation (on my computer)
fn required_layer_names<'a>(entry: &ash::Entry) -> ValidationInfo<'a> { //I think this means rust will deallocate the struct (and therefore the &str) once the function is done
	let available_layers: Vec<vk::LayerProperties> = unsafe {entry.enumerate_instance_layer_properties().expect("Failed to enumerate layers")};
	let available_layer_names :Vec<&str> = available_layers
		.iter()
		.map(|layer_properties| layer_properties.layer_name_as_c_str().unwrap().to_str().unwrap())
		.collect();
	
	//Check the listed validation layers against the instance, panic if one isn't available. Then returns the shit.
	for layer_to_check in VALIDATION_LAYERS {
		if !available_layer_names.contains(&layer_to_check) && VALIDATION_ENABLED {
			panic!("Validation layer not available")
		}
	}
	
	//Return the validation bullshit
	ValidationInfo {
		is_enabled: VALIDATION_ENABLED,
		validation_layers: VALIDATION_LAYERS
	}	  
}

//Queue family will finds/hold the indices for the queue with whatever command types we want
struct QueueFamilyIndices {
	graphics_family: Option<u32>,
	present_family: Option<u32>,
}

impl QueueFamilyIndices {
	fn find_queue_families(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq) -> QueueFamilyIndices{
		//Get the queues of the physical device (same way it's done in "physical_device_suitability_score")
		//There's some default structs happening here - vulkan overwrites these structs. I think ash is doing something kinda quirky here to deal with rust/vulkan??
		let queue_family_count_len = unsafe { instance.get_physical_device_queue_family_properties2_len(physical_device) }; //Finds the length (# of available queue families)
		let mut queue_families = vec![vk::QueueFamilyProperties2 {..Default::default()}; queue_family_count_len]; //Need to pass the length in here for ash, which is why the line above is included
		unsafe { instance.get_physical_device_queue_family_properties2(physical_device, &mut queue_families) }; //Queue family properties - what the queue families supported by the device can do (vector of families)

		//Initialize with nothing for the indices
		let mut queue_family_index = QueueFamilyIndices {
			graphics_family: None,
			present_family: None,
		};


		//If a queue family is found that can support graphics (or whatever other type we want, given the appropriate if statements), AND there's a higher queue count than the current one, send it
		let mut graphics_queue_count_at_index = 0;
		for (i, family) in queue_families.iter().enumerate() {
			//This if statement checks the current queue family for graphics support
			if family.queue_family_properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) && family.queue_family_properties.queue_count > graphics_queue_count_at_index {
				queue_family_index.graphics_family = Some(i as u32);
				graphics_queue_count_at_index = family.queue_family_properties.queue_count;
			}

			//Get a bool for if the queue family supports presentation
			let presentation_support_bool = unsafe { surface_req.surface_loader.get_physical_device_surface_support(physical_device, i as u32, surface_req.surface).unwrap() };

			//Do the same for the present queue. Prefer it to be the same as the graphics queue if possible
			//If the current queue family is the first one that supports presentation, set it
			//For future iterations in the for loop, if current family supports presentation and is the same as the graphics queue family index, set it (should work since graphics queue family index is set first)
			if presentation_support_bool && (queue_family_index.present_family.is_none() || Some(i as u32) == queue_family_index.graphics_family) {
				queue_family_index.present_family = Some(i as u32);
			}
		}

		//Return the updated indices
		queue_family_index
	}
}

//Make a struct to hold a surface and its loader (instance for the surface extension)
//Just makes it less wordy passing surfaces into fns
struct SurfaceReq {
	surface_loader: khr::surface::Instance,
	surface: vk::SurfaceKHR
}

//Another extension comes with another required instance
struct SwapchainReq {
	swapchain_loader: khr::swapchain::Device, //There's also one instance level function, but I don't think I'll need it. If I do, I can add like a "swapchain_loader_instance" field
	swapchain: vk::SwapchainKHR,
	swapchain_format: vk::SurfaceFormatKHR,
	swapchain_extent: vk::Extent2D,
	swapchain_images: Vec<vk::Image>
}

//Need to check: surface capabilities, surface formats, and presentation modes
struct SwapchainSupportDetails {
	capabilities: vk::SurfaceCapabilitiesKHR,
	formats: Vec<vk::SurfaceFormatKHR>,
	present_modes: Vec<vk::PresentModeKHR>
}

impl SwapchainSupportDetails {
	//Populates a "SwapchainSupportDetails" struct
	fn query_swapchain_support_details(physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq) -> SwapchainSupportDetails {
		//Get the physical device's supported capabilities, formats, and present modes for the swapchain
		//There's a "2" version of these funcs, but it isn't relevant to me. It also is a whole nother extension, which means a whole nother instance because of ash.
		let capabilities = unsafe { surface_req.surface_loader.get_physical_device_surface_capabilities(physical_device, surface_req.surface).unwrap() };
		let formats = unsafe { surface_req.surface_loader.get_physical_device_surface_formats(physical_device, surface_req.surface).unwrap() };
		let present_modes = unsafe { surface_req.surface_loader.get_physical_device_surface_present_modes(physical_device, surface_req.surface).unwrap() };
		
		//Make and return the SwapchainSupportDetails struct
		SwapchainSupportDetails {
			capabilities,
			formats,
			present_modes
		}
	}
}









//A bunch of shit is gonna go in here
pub struct VulkanApp {
	entry: ash::Entry, //I think the entry just lets you use all the functions without needing an instance
	instance: ash::Instance, //The instance of vulkan - does uhhhhh everything
	
	surface: vk::SurfaceKHR, //Vulkan surface that gets rendered to
	surface_loader: khr::surface::Instance,
	
	physical_device: vk::PhysicalDevice, //Physical device - the GPU
	device: ash::Device, //Logical device - one instance of vulkan run on the physical device

	graphics_queue: vk::Queue, //Queue - where graphics operations are run
	present_queue: vk::Queue, //Queue that has presentation support (likely the same as the graphics queue, but not necessarily)

	swapchain:vk::SwapchainKHR, //Swapchain - handles screen display + vsync/buffering
	swapchain_loader: khr::swapchain::Device,
	swapchain_image_views: Vec<vk::ImageView> //Image views that describe image access for all the images on the swapchain

}

//OpenGLcels seething over Vulkanchads
impl VulkanApp {
	//Initializes VulkanApp with an instance
	pub fn init_vulkan(window: &Window) -> VulkanApp {
		//Make an entry. Seems like this is just ash's thing to call functions before an instance is created.
		let entry = Entry::linked();

		//Need to do the strings as cstrings. Yay!
		let app_name = CString::new(WINDOW_TITLE).unwrap(); //Just use window title for the app name
		let engine_name = CString::new("jarmungular_ engine").unwrap();

		//Application info - gotta feed this to the instance info (create_info below)
		let app_info = vk::ApplicationInfo {
			s_type: vk::StructureType::APPLICATION_INFO, //s_type is just the structure type here. I guess it becomes important if using p_next
			p_next: ptr::null(), //This is used to do linked lists between structures, used for extensions + memory stuff I guess
			p_application_name: app_name.as_ptr(), //They need to go in here as "*const c_char," per the ash documentation. This is just a raw pointer to a c_char.
			application_version: vk::make_api_version(0, 1, 0, 0), //Version of the application I'm making
			p_engine_name: engine_name.as_ptr(), //I guess we doin pointers now
			engine_version: vk::make_api_version(0, 1, 0, 0), //Version of the engine I'm making
			api_version: vk::API_VERSION_1_3, //This is a const in vulkan for the version 1.3. Tutorial uses 1.0, we'll see what happens
			..Default::default() //There's also _marker. No clue what that does, it's an ash thing
		};

		
		//Get extensions from khr (defined up at the top, might move it into a new file later)
		let extension_names = required_extension_names(&entry);

		//Set up validation layers. If is_enabled is false, these will get zorped later. I tried to put this whole thing in an if statement and it was a pain so whatev
		let validation_layer_raw_names: Vec<CString> = required_layer_names(&entry)
			.validation_layers
			.iter()
			.map(|layer_name| CString::new(*layer_name).unwrap())
			.collect();
		let enable_layer_names: Vec<*const i8> = validation_layer_raw_names
			.iter()
			.map(|layer_name| layer_name.as_ptr())
			.collect();


		//Instance creation info to feed the create_instance function
		let create_info = vk::InstanceCreateInfo {
			s_type: vk::StructureType::INSTANCE_CREATE_INFO,
			p_next: ptr::null(), //This is to extend the struct, used for extensions/debug callbacks I think
			flags: vk::InstanceCreateFlags::empty(), //Don't need to deal with any flags (which is good because idk what they'd be used for)
			p_application_info: &app_info,
			pp_enabled_layer_names: if VALIDATION_ENABLED{enable_layer_names.as_ptr()} else {ptr::null()}, //Global validation layers. Only use if enabled, otherwise ignore
			enabled_layer_count: if VALIDATION_ENABLED{enable_layer_names.len() as u32} else {0},
			pp_enabled_extension_names: extension_names.as_ptr(), //Global extensions - apply to the entire program, not just a specific device
			enabled_extension_count: extension_names.len() as u32,
			..Default::default() //There's also _marker. I think it has to do with lifetimes but idk
			
		};

		//Create the instance
		let instance = unsafe { entry.create_instance(&create_info, None).expect("Failed to create the instance") };
		//Setup the surface
		let surface_req = VulkanApp::create_surface(&entry, &instance, window);
		//Create the physical device
		let physical_device = VulkanApp::select_physical_device(&instance, &surface_req);
		//Create logical device and graphics/present queue
		let (device, graphics_queue, present_queue) = VulkanApp::create_logical_device(&instance, physical_device, &surface_req, enable_layer_names);
		//Create swapchain (and all the fun stuff that comes with it)
		let swapchain_req = VulkanApp::create_swapchain(&instance, &device, physical_device, &surface_req);
		//Create image views for all the swapchain images
		let swapchain_image_views = VulkanApp::create_image_views(&device, swapchain_req.swapchain_format, &swapchain_req.swapchain_images);
		//Create a pipeline including the vertex/fragment shaders
		VulkanApp::create_pipeline(&device);


		//Now stick those into the VulkanApp fields to initiate everything (returns this struct)
		VulkanApp {
			entry,
			instance,
			surface: surface_req.surface,
			surface_loader: surface_req.surface_loader,
			physical_device,
			device,
			graphics_queue,
			present_queue,
			swapchain: swapchain_req.swapchain,
			swapchain_loader: swapchain_req.swapchain_loader,
			swapchain_image_views
		}
	}

	//Selects the physical device (the GPU) that vulkan uses 
	fn select_physical_device(instance: &ash::Instance, surface_req: &SurfaceReq) -> vk::PhysicalDevice {
		let physical_devices = unsafe { instance.enumerate_physical_devices().expect("Failed to enumerate physical devices") };
		println!("{:?} device(s) with vulkan support", physical_devices.len());

		//Now check for the most suitable physical device using "physical_device_suitablility" function
		let mut physical_devices_scored: Vec<(vk::PhysicalDevice, u32)> = physical_devices.into_iter()
			.map(|physical_device| (physical_device, VulkanApp::physical_device_suitability_score(instance, physical_device, surface_req))) //Maps to a tuple of the device and its score
			.collect::<Vec<(vk::PhysicalDevice, u32)>>();

		//Sort so the best one is first
		physical_devices_scored.sort_by_key(|k| k.1);

		//Check if the best one is okay (more than zero)
		if physical_devices_scored[physical_devices_scored.len()-1].1 == 0 {panic!("No suitable devices detected")};

		physical_devices_scored[physical_devices_scored.len()-1].0 //Just returns the device, not score
	}

	//This just checks each physical device and gives a score based on properties/features
	fn physical_device_suitability_score(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq) -> u32 {
		//These "2"s are all from an update that added pnext to the functions. I really don't need them, but I'm using them anyway.
		//They all take the old v1 versions of the structs before doing their thing - that's what's behind the "defaults" above, and needs to be expressly stated for the queue families (to get the amount of queue families)
		//The "get_physical_device_properties" function will modify this, so just set it default for now
		//Future Jake says: on a second look, I really didn't need to do this. This is only used to add new extensions to the property/feature getters, which I'm not using.
		let mut device_properties = vk::PhysicalDeviceProperties2 {
			..Default::default()
		};
		//Do the same for physical device features
		let mut device_features = vk::PhysicalDeviceFeatures2 {
			..Default::default()
		};
		//And for queue families. First need to find out how many queue families there are though.
		let queue_family_count_len = unsafe { instance.get_physical_device_queue_family_properties2_len(physical_device) };
		let mut queue_families = vec![vk::QueueFamilyProperties2 {..Default::default()}; queue_family_count_len];

		unsafe { instance.get_physical_device_properties2(physical_device, &mut device_properties) }; //Physical device properties - name, id, max buffers, etc
		unsafe { instance.get_physical_device_features2(physical_device, &mut device_features) }; //Physical device features - cool and fun features
		unsafe { instance.get_physical_device_queue_family_properties2(physical_device, &mut queue_families) }; //Queue family properties - what the queue families supported by the device can do (vector of families)

		//Making the score a u32, so gotta be sure to only add, not subtract (have a score >=0, 0 if unsupported)
		let mut score: u32 = 0;

		//Check discrete/integrated GPU, prefer discrete
		if device_properties.properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
			score += 10;
		}
		if device_properties.properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
			score += 5;
		}

		//Check indices - make sure the device has the queue families we want
		let queue_family_indices = QueueFamilyIndices::find_queue_families(instance, physical_device, surface_req);
		if queue_family_indices.graphics_family.is_none() {score = 0;}
		if queue_family_indices.present_family.is_none() {score = 0;}

		//Check if the device supports all extensions in the "DEVICE_EXTENSIONS" const (swapchain extension, for ex)
		if !VulkanApp::check_device_extension_support(instance, physical_device) {score = 0}

		//Check swapchain support stuff. If there isn't at least one supported image format/presentation mode, it's over
		let swapchain_support_details = SwapchainSupportDetails::query_swapchain_support_details(physical_device, surface_req);
		if swapchain_support_details.formats.is_empty() || swapchain_support_details.present_modes.is_empty() {score = 0}

		//Print some info (this is all debug style stuff, probably useful though)
		let device_name = device_properties.properties.device_name_as_c_str().unwrap();
		let device_id = device_properties.properties.device_id;
		let device_type = device_properties.properties.device_type;
		let api_version_major = vk::api_version_major(device_properties.properties.api_version);
		let api_version_minor = vk::api_version_minor(device_properties.properties.api_version);
		let api_version_patch = vk::api_version_patch(device_properties.properties.api_version);

		println!("Device: {:?}, ID: {:?}, Type: {:?}, JAKESCORE: {:?}", device_name, device_id, device_type, score);
		println!("API version: {:?}.{:?}.{:?}", api_version_major, api_version_minor, api_version_patch);

		println!("Queue Families:");
		for family in &queue_families {
			println!("\tQueue count: {:?}, Flags: {:?}", family.queue_family_properties.queue_count, family.queue_family_properties.queue_flags);
		}

		//Return the calculated score
		score
	}

	//Checks if a given physical device supports all extensions in the "DEVICE_EXTENSIONS" const
	fn check_device_extension_support(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> bool {
		//Get all the supported device extensions
		let supported_extensions = unsafe { instance.enumerate_device_extension_properties(physical_device).expect("Couldn't get supported device extensions") };

		//Same as the thing from "required_extension_names" function
		let supported_extension_names :Vec<&CStr> = supported_extensions
			.iter()
			.map(|extension_properties| extension_properties.extension_name_as_c_str().unwrap())
			.collect();

		//Go through and check all the device extensions
		let mut extensions_supported_bool = true;
		for extension_to_check in &DEVICE_EXTENSIONS {
			if !supported_extension_names.contains(extension_to_check) {
				extensions_supported_bool = false;
			}
		}

		// Print some debug info
		println!("Supported device extensions:");
		// for extension_properties in supported_extensions {
		//   print!("{:?}, ", extension_properties.extension_name_as_c_str().unwrap());
		// }
		// println!();

		//Return the bool
		extensions_supported_bool
	}

	//Find a graphics queue family, create the logical device, create queue
	fn create_logical_device(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq, validation: Vec<*const i8>) -> (ash::Device, vk::Queue, vk::Queue) {
		//Get the queue family indices
		let queue_family_indices = QueueFamilyIndices::find_queue_families(instance, physical_device, surface_req);

		//Get UNIQUE queue family indices
		//This would be more efficient with a hashset, but this should only deal with a few familiy indices so doesn't really matter
		let mut unique_queue_family_indices = vec![queue_family_indices.graphics_family, queue_family_indices.present_family];
		unique_queue_family_indices.sort();
		unique_queue_family_indices.dedup(); //Zorps duplicates (needed to be sorted first)
		
		//For now, just want to create 1 queue from each queue family (with priority 1.0)
		//Could probably play around with this and make the "unique_family_queue_indices" a tuple for each queue family that includes the queue count
		let queue_counts = 1_u32; //# of queues being created from the given queue family
		let queue_priorities = [1.0f32]; //There will be one for each queue being created from the queue family
		
		//The device queue creation info will feed into the device creation info
		//Each iteration corresponds to one queue family
		//For now, just taking the same count and priority for each queue in the queue family
		let mut queue_create_info_vec = vec![];
		for index in unique_queue_family_indices {
			let queue_create_info = vk::DeviceQueueCreateInfo {
				s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::DeviceQueueCreateFlags::empty(),
				queue_family_index: index.unwrap(),
				queue_count: queue_counts,
				p_queue_priorities: queue_priorities.as_ptr(),
				..Default::default()
			};
			queue_create_info_vec.push(queue_create_info);
		}

		//Physical device features will also feed into the device info - not using any for now
		let physical_device_features = vk::PhysicalDeviceFeatures {
			..Default::default()
		};

		//Set up device specific validation layers as well. Validation layers are deprecated for logical devices, but gotta stick em in anyway
		//Just take them pre-baked from the "init_vulkan" function as "validation" in the function call
		//(I commented them out teehee, so "validation" in the fn call is dead code right now)
		//No device specific extensions for now

		//Now do the device creation info (logical device)
		let device_info = vk::DeviceCreateInfo {
			s_type: vk::StructureType::DEVICE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DeviceCreateFlags::empty(),
			queue_create_info_count: queue_create_info_vec.len() as u32, //Length of the vector with all the queue create infos
			p_queue_create_infos: queue_create_info_vec.as_ptr(),
			//enabled_layer_count: if VALIDATION_ENABLED{validation.len() as u32} else {0}, //DEPRECATED
			//pp_enabled_layer_names: if VALIDATION_ENABLED{validation.as_ptr()} else {ptr::null()}, //DEPRECATED
			enabled_extension_count: DEVICE_EXTENSIONS.len() as u32, //Enable extensions from the const "DEVICE EXTENSIONS"
			pp_enabled_extension_names: &DEVICE_EXTENSIONS[0].as_ptr(),
			p_enabled_features: &physical_device_features,
			..Default::default()
		};

		//Now create the device
		let device = unsafe {instance.create_device(physical_device, &device_info, None).expect("Logical device creation failed") };

		//Now I want to return the graphics queue, so I need to get its info (different than the "DeviceQueueCreateInfo" struct)
		let graphics_queue_info = vk::DeviceQueueInfo2 { //Will use this in "get_device_queue2"
			s_type: vk::StructureType::DEVICE_QUEUE_INFO_2,
			p_next: ptr::null(),
			flags: vk::DeviceQueueCreateFlags::empty(),
			queue_family_index: queue_family_indices.graphics_family.unwrap(),
			queue_index: 0, //Since there's only one graphics queue, this works. Otherwise, I think I'd have to get the info for all of them separately
			..Default::default()
		};

		//Do the same for the queue with presentation support
		let present_queue_info = vk::DeviceQueueInfo2 { //Will use this in "get_device_queue2"
			s_type: vk::StructureType::DEVICE_QUEUE_INFO_2,
			p_next: ptr::null(),
			flags: vk::DeviceQueueCreateFlags::empty(),
			queue_family_index: queue_family_indices.present_family.unwrap(),
			queue_index: 0, //Since there's only one presentation queue, this works. Otherwise, I think I'd have to get the info for all of them separately
			..Default::default()
		};

		//Get info of the device's queue at the specified index/length above
		let graphics_queue = unsafe { device.get_device_queue2(&graphics_queue_info) };
		let present_queue = unsafe { device.get_device_queue2(&present_queue_info) };

		//Return the device and graphics queue in a tuple
		(device, graphics_queue, present_queue)
	}

	//Create the surface to display to (WINDOWS ONLY AT THE MOMENT)
	//This will be called in the application handler when the vulkan app is "resumed"
	fn create_surface(entry: &ash::Entry, instance: &ash::Instance, window: &Window) -> SurfaceReq {
		//First get the hinstance and hwnd from winit window
		let raw_window_handle = window.window_handle().expect("Couldn't get window handle").as_raw();
		let (hinstance, hwnd) = match window.window_handle().unwrap().as_raw() {
			RawWindowHandle::Win32(handle) => (handle.hinstance.expect("Couldn't get hinstance").get(), handle.hwnd.get()),
			_ => panic!("WINDOWS ONLY, FOOL")
		};

		//Surface creation info:
		let surface_info = vk::Win32SurfaceCreateInfoKHR {
			s_type: vk::StructureType::WIN32_SURFACE_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::Win32SurfaceCreateFlagsKHR::empty(),
			hinstance,
			hwnd,
			..Default::default()
		};

		//Creates the loader instance for the win32 window. I think this is an ash thing, where the extension has to have its own separate instance.
		let surface_loader_win32 = khr::win32_surface::Instance::new(entry, instance);
		//Also need the more general surface loader to destroy shit later
		let surface_loader = khr::surface::Instance::new(entry, instance);
		//Create surface
		let surface = unsafe { surface_loader_win32.create_win32_surface(&surface_info, None).expect("Failed to create surface") };
		
		//Return the surface and its instance
		SurfaceReq {
			surface,
			surface_loader
		}
	}

	//Chooses the surface format for swapchain creation
	fn choose_swapchain_format(available_formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
		let fallback = available_formats[0];
		for available_format in available_formats {
			if available_format.format == vk::Format::R8G8B8A8_SRGB && available_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
				return available_format
			}
		}
		//If the most common srgb format (R8G8B8A8_SRGB) isn't available, just return the first one in the format vec
		fallback
	}

	//Choose the presentation mode for swapchain creation
	//This is where vsync stuff is decided. Vsync off would be "IMMEDIATE"
	//Prefer mailbox mode for now. Higher cpu/gpu load, but less latency?? (NEEDS TEST) and no tearing (good middle ground)
	//Mailbox does "drop frames" as opposed to FIFO which slows stuff down. I kinda like that, but we will see if it needs to be changed
	fn choose_presentation_mode(available_present_modes: Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
		let fallback = vk::PresentModeKHR::FIFO;
		for available_present in available_present_modes {
			if available_present == vk::PresentModeKHR::MAILBOX {
				return available_present
			}
		}
		//Go for FIFO if mailbox isn't available. FIFO is guaranteed to be supported, as long as presentation is supported (which is assumed at this point)
		fallback
	}

	//This gives the size of the swapchin in pixels
	//Clamp it to our min/max from the capabilities
	//Not optimized compared to vulkan-tutorial-rust. They do a check for an unbounded width/height first, but whatev
	fn choose_swapchain_extent(capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
		let mut width = WINDOW_WIDTH;
		let mut height = WINDOW_HEIGHT;

		//Clamp!! There's a clamp function num::clamp but I don't wanna use it teehee
		if width < capabilities.min_image_extent.width {
			width = capabilities.min_image_extent.width;
		}
		if width > capabilities.max_image_extent.width {
			width = capabilities.max_image_extent.width;
		}
		if height < capabilities.min_image_extent.height {
			height = capabilities.min_image_extent.height;
		}
		if height > capabilities.max_image_extent.height {
			height = capabilities.max_image_extent.height;
		}

		//Return an Extent2D
		vk::Extent2D {
			width,
			height
		}
	}

	//Use all the "chooser" functions then make the swapchain
	fn create_swapchain(instance: &ash::Instance, device: &ash::Device, physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq) -> SwapchainReq {
		//Get all the fun info that's required for swapchain creation
		let swapchain_support_details = SwapchainSupportDetails::query_swapchain_support_details(physical_device, surface_req);
		let surface_format = VulkanApp::choose_swapchain_format(swapchain_support_details.formats);
		let present_mode = VulkanApp::choose_presentation_mode(swapchain_support_details.present_modes);
		let extent = VulkanApp::choose_swapchain_extent(swapchain_support_details.capabilities);

		//Swapchain image count. Seems like the driver may hijack some in some cases, making more than the requested minimum.
		//This is relevant for double/triple buffering if in FIFO.
		//Mailbox is kinda like a "fast triple buffering" option. It SHOULD have the latency of double buffering, while also displaying the most recently presented image (no game slowdown on lag).
		//It kinda makes the whole double/triple buffer (which, for FIFO, is determined by image count) irrelevant?
		//Look at api-without-secrets-introduction-to-vulkan-part-2 pg 25 for more.
		//If mailbox, go with minimum supported + 1
		let image_count: u32 = if present_mode == vk::PresentModeKHR::MAILBOX {
			swapchain_support_details.capabilities.min_image_count + 1
		}
		//If FIFO, just go with 2 (or the minimum supported)
		else if present_mode == vk::PresentModeKHR::FIFO && swapchain_support_details.capabilities.min_image_count >= 2 {
			2
		}
		//This should be exhaustive, since present mode will either be mailbox or FIFO according to the "choose_presentation_mode" fn
		else {
			swapchain_support_details.capabilities.min_image_count
		};

		//Need to handle the case of graphics queue family being different than presentation queue family
		//In that case, the swapchain will need to be shared between the two families
		//As an optimization, could reuse these from the logical device creation, pass them in here. Would need to create the device's queues in "init_vulkan"
		let queue_family_indices = QueueFamilyIndices::find_queue_families(instance, physical_device, surface_req);
		let (image_sharing_mode, queue_family_index_count, queue_family_indices) = if queue_family_indices.graphics_family == queue_family_indices.present_family {
			(vk::SharingMode::EXCLUSIVE, 0, vec![])
		} else {
			(vk::SharingMode::CONCURRENT, 2, vec![queue_family_indices.graphics_family.unwrap(), queue_family_indices.present_family.unwrap()])
		};

		let swapchain_info = vk::SwapchainCreateInfoKHR {
			s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: surface_req.surface,
			min_image_count: image_count,
			image_format: surface_format.format,
			image_color_space: surface_format.color_space,
			image_extent: extent,
			image_array_layers: 1, //Used for stereoscopic 3d, not doing that so it's just 1
			image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT, //This defines what the swapchain images will be used for. This one allows the image view created from this image to be used as the color attachment of a framebuffer
			image_sharing_mode,
			queue_family_index_count,
			p_queue_family_indices: queue_family_indices.as_ptr(),
			pre_transform: swapchain_support_details.capabilities.current_transform,
			composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE, //Used for blending between different windows. Don't want that, only using one window, just make it opaque
			present_mode,
			clipped: vk::TRUE, //Means we don't really care about the color of pixels behind other windows
			old_swapchain: vk::SwapchainKHR::null(), //This apparently helps with resource reuse if an old swapchain exists
			..Default::default()
		};
		
		//Create the loader
		let swapchain_loader = khr::swapchain::Device::new(instance, device);

		//Create the swapchain
		let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_info, None).expect("Failed to create swapchain") };

		//Get the swapchain images
		let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };

		//Return the swapchain_req
		SwapchainReq {
			swapchain_loader,
			swapchain,
			swapchain_format: surface_format,
			swapchain_extent: extent,
			swapchain_images
		}
	}

	//Create image views for the swapchain images
	//Input a vec of images from swapchain_req.swapchain_images
	fn create_image_views(device: &ash::Device, surface_format: vk::SurfaceFormatKHR, images: &Vec<vk::Image>) -> Vec<vk::ImageView> { //DELETE REFERENCE FOR SURFACE FORMAT?? ASK CLIPPY
		let mut swapchain_image_views = vec![];
		//Loop over all the images and get image views
		for &image in images {
			//Make an image view creation info struct for each one of the images
			let image_view_info = vk::ImageViewCreateInfo {
				s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::ImageViewCreateFlags::empty(),
				image,
				view_type: vk::ImageViewType::TYPE_2D_ARRAY, //Can treat as a 1d texture, 2d texture, or 3d texture/cube map
				format: surface_format.format, //Get from the surface format passed into the function
				components: vk::ComponentMapping { //No swizzle wanted right now, so just map components as-is
					r: vk::ComponentSwizzle::R,
					g: vk::ComponentSwizzle::G,
					b: vk::ComponentSwizzle::B,
					a: vk::ComponentSwizzle::A,
				},
				subresource_range: vk::ImageSubresourceRange { //Sets the image aspects that will be included in the image view, we only care about color. Also deals with mipmaps/layers
					aspect_mask: vk::ImageAspectFlags::COLOR,
					base_mip_level: 0, //No mipmapping right now, just going to have 1 mip level
					level_count: 1,
					base_array_layer: 0, //Used for stereographic shit, swapchain images (and pretty much all other) just have 1 array layer (defined in "create_swapchain" fn)
					layer_count: 1
				}, 
				..Default::default()
			};

			//Create the image view for the image being iterated over and push it into "swapchain_image_views" vec
			let swapchain_image_view = unsafe { device.create_image_view(&image_view_info, None).expect("Failed to create image view") };
			swapchain_image_views.push(swapchain_image_view);
		}
		//Return the vec of image views
		swapchain_image_views
	}

	//Create the pipeline
	fn create_pipeline(device: &ash::Device) {
		//Read the spirv files for teh vertex/fragment shaders
		//Shader modules should be destroyed after pipeline creation
		let fragment_shader_code = r_shader("./src/render/shaders/fragment.spv");
		let vertex_shader_code = r_shader("./src/render/shaders/vertex.spv");

		//Create the shader modules from those files
		let fragment_shader_module = VulkanApp::create_shader_module(device, fragment_shader_code);
		let vertex_shader_module = VulkanApp::create_shader_module(device, vertex_shader_code);
		
		//Define the shader entry point - the function of the shader that will run. We want "main" to run (the only one in there at the moment)
		//Have to define here because rust doesn't like the lifetime of the pointer dying so quickly
		let shader_entry_point = CString::new("main").unwrap();

		//Pipeline fragment shader stage
		let pipeline_fragment_stage_info = vk::PipelineShaderStageCreateInfo {
		    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
		    p_next: ptr::null(),
		    flags: vk::PipelineShaderStageCreateFlags::empty(), //Subgroup flags - has to do with sharing data. Don't care at the moment
		    stage: vk::ShaderStageFlags::FRAGMENT, //Fragment shader flag
		    module: fragment_shader_module, //The shader module yay yay yippee
		    p_name: shader_entry_point.as_ptr(), //Entry point of the shader
		    p_specialization_info: ptr::null(), //This is used to specify shader constants before render time. Ex: fragment shader where material 1 has "ploopyness = 50", material 2 has "ploopyness = 100"
		    ..Default::default()
		};

		//Pipeline vertex shader stage
		let pipeline_vertex_stage_info = vk::PipelineShaderStageCreateInfo {
		    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
		    p_next: ptr::null(),
		    flags: vk::PipelineShaderStageCreateFlags::empty(),
		    stage: vk::ShaderStageFlags::VERTEX, //Vertex shader flag
		    module: vertex_shader_module,
		    p_name: shader_entry_point.as_ptr(),
		    p_specialization_info: ptr::null(),
		    ..Default::default()
		};

		//Make an array containing the two pipeline shader stage infos
		let shader_stages = [pipeline_fragment_stage_info, pipeline_vertex_stage_info];

		//Destroy the shader modules now
		unsafe {
			device.destroy_shader_module(vertex_shader_module, None);
			device.destroy_shader_module(fragment_shader_module, None);
		}
	}

	//Create shader modules to be used in pipeline
	fn create_shader_module(device: &ash::Device, shader_code: Vec<u8>) -> vk::ShaderModule {
		//Shader module creation info
		let shader_module_info = vk::ShaderModuleCreateInfo {
			s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
    		p_next: ptr::null(),
    		flags: vk::ShaderModuleCreateFlags::empty(),
    		code_size: shader_code.len(), //Length in bytes of the shader code
    		p_code: shader_code.as_ptr() as *const u32, //Shader code (must be in spirv format)
    		..Default::default()
		};

		//Now create + return the shader module
		unsafe { device.create_shader_module(&shader_module_info, None).expect("Couldn't create shader module") }
	}
}

//Have to destroy anything that was explicitly created
impl Drop for VulkanApp {
	fn drop(&mut self) {
		unsafe {
			for swapchain_image_view in &self.swapchain_image_views {
				self.device.destroy_image_view(*swapchain_image_view, None);
			}
			self.swapchain_loader.destroy_swapchain(self.swapchain, None);
			self.device.destroy_device(None);
			self.surface_loader.destroy_surface(self.surface, None);
			self.instance.destroy_instance(None);
		}
	}
}