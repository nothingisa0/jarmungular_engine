use crate::render::constants::*;
use crate::render::memory::{create_buffer, copy_buffer};
use crate::render::Vertex;
use crate::scene::{Scene, TEST_TRIANGLE_VERTICES};
use crate::utility::read::{fragment_shader, vertex_shader};

use std::ptr;
use std::ffi::{CString, CStr};
use ash::{vk, khr, Entry};
use winit::{
	window::{Window},
	raw_window_handle::{HasWindowHandle, RawWindowHandle},
};


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
	swapchain_image_views: Vec<vk::ImageView>, //Image views that describe image access for all the images on the swapchain
	swapchain_framebuffers: Vec<vk::Framebuffer>, //Framebuffers define the attachments to be written to (image views)
	swapchain_extent: vk::Extent2D, //The size of the swapchain images

	render_pass: vk::RenderPass, //Describes framebuffer attachments and subpasses for the pipeline
	pipeline: vk::Pipeline, //A graphics pipeline with all the shaders + fixed functions in there
	pipeline_layout: vk::PipelineLayout, //Deals with descriptor sets and push constants for pipeline to access

	vertex_buffer: vk::Buffer, //Buffer used to hold all the juicy vertex data
	vertex_buffer_memory: vk::DeviceMemory, //The memory the vertex buffer is allocated to

	command_pool: vk::CommandPool, //Deals with memory stuff for the command buffers
	command_pool_short: vk::CommandPool, //Command buffers created from this pool will be short lived
	command_buffers: Vec<vk::CommandBuffer>, //Records commands which are then submitted to a queue

	//Synchronization objects
	//Semaphore - used for GPU-GPU synchronization
	//Fence - used for CPU-GPU synchronization
	image_available_semaphore: vk::Semaphore, //Signals that an image has been acquired from the swapchain and is ready for rendering
	render_finished_semaphore: vk::Semaphore, //Signals that rendering is finished, presentation can happen
	in_flight_fence: vk::Fence, //Signals that a frame is in flight

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
		//Find the queue family indices from the physicl device. These will be used for the logical device creation (and a couple other things after that)
		let queue_family_indices = QueueFamilyIndices::find_queue_families(&instance, physical_device, &surface_req);
		//Create logical device and graphics/present queue
		let (device, graphics_queue, present_queue) = VulkanApp::create_logical_device(&instance, physical_device, &queue_family_indices, &surface_req, enable_layer_names);
		//Create swapchain (and all the fun stuff that comes with it)
		let swapchain_req = VulkanApp::create_swapchain(&instance, &device, physical_device, &surface_req, &queue_family_indices, WINDOW_WIDTH, WINDOW_HEIGHT);
		//Create image views for all the swapchain images
		let swapchain_image_views = VulkanApp::create_image_views(&device, swapchain_req.swapchain_format, &swapchain_req.swapchain_images);
		//Create the render pass
		let render_pass = VulkanApp::create_render_pass(&device, swapchain_req.swapchain_format.format);
		//Create a pipeline including the vertex/fragment shaders
		let (pipeline, pipeline_layout) = VulkanApp::create_pipeline(&device, render_pass, swapchain_req.swapchain_extent);
		//Create the framebuffers that contain the image views for the swapchain images as attachments
		let swapchain_framebuffers = VulkanApp::create_framebuffers(&device, render_pass, &swapchain_image_views, swapchain_req.swapchain_extent);
		//Create the command pool for the graphics family. Also create a short lived command pool on the graphics family for one time operations
		let (command_pool, command_pool_short) = VulkanApp::create_command_pools(&device, &queue_family_indices);
		//Create the command buffer with all the recorded commands
		let command_buffers = VulkanApp::create_command_buffer(&device, command_pool);
		//Create the vertex buffer
		let (vertex_buffer, vertex_buffer_memory) = VulkanApp::create_vertex_buffer(&instance, &device, physical_device, command_pool_short, graphics_queue);
		//Create all the stuff needed to synchronize the draw
		let (image_available_semaphore, render_finished_semaphore, in_flight_fence) = VulkanApp::create_sync_objects(&device);

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
			swapchain_image_views,
			swapchain_framebuffers,
			swapchain_extent: swapchain_req.swapchain_extent,

			render_pass,
			pipeline,
			pipeline_layout,

			command_pool,
			command_pool_short,
			command_buffers,

			vertex_buffer,
			vertex_buffer_memory,

			image_available_semaphore,
			render_finished_semaphore,
			in_flight_fence,
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
	fn create_logical_device(instance: &ash::Instance, physical_device: vk::PhysicalDevice, queue_family_indices: &QueueFamilyIndices, surface_req: &SurfaceReq, validation: Vec<*const i8>) -> (ash::Device, vk::Queue, vk::Queue) {
		//Passing the queue family indices into this function, since they're used for a few other things as well
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

	//Use all the "chooser" functions to make the swapchain
	fn create_swapchain(instance: &ash::Instance, device: &ash::Device, physical_device: vk::PhysicalDevice, surface_req: &SurfaceReq, queue_family_indices: &QueueFamilyIndices, window_width: u32, window_height: u32) -> SwapchainReq {
		//Get all the fun info that's required for swapchain creation
		let swapchain_support_details = SwapchainSupportDetails::query_swapchain_support_details(physical_device, surface_req);
		let surface_format = VulkanApp::choose_swapchain_format(swapchain_support_details.formats);
		let present_mode = VulkanApp::choose_presentation_mode(swapchain_support_details.present_modes);
		let extent = VulkanApp::choose_swapchain_extent(swapchain_support_details.capabilities, window_width, window_height);

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
		//Queue family indices are passed into this function
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
	//Need to consider frames in flight with all these options as well
	fn choose_presentation_mode(available_present_modes: Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
		let mut present_mode_flag = 0b0000_0000;
		//Check for desired present modes, set a flag
		for available_present in available_present_modes {
			if available_present == vk::PresentModeKHR::MAILBOX {
				present_mode_flag |= 0b0000_0010;
			} else if available_present == vk::PresentModeKHR::IMMEDIATE {
				present_mode_flag |= 0b0000_0001;
			} else if available_present == DESIRED_PRESENTATION_MODE {
				present_mode_flag |= 0b0000_0100;
			}
		}
		//Check the flag, go in priority order desired -> mailbox -> immediate -> fifo
		if present_mode_flag > 0b0000_0010 {
			DESIRED_PRESENTATION_MODE
		}
		else if present_mode_flag > 0b0000_0001 {
			vk::PresentModeKHR::MAILBOX
		} else if present_mode_flag > 0b0000_0000 {
			vk::PresentModeKHR::IMMEDIATE
		} else {
			vk::PresentModeKHR::FIFO
		}
	}

	//This gives the size of the swapchin in pixels
	//Clamp it to our min/max from the swapchain's capabilities
	//Not optimized compared to vulkan-tutorial-rust. They do a check for an unbounded width/height first, but whatev
	fn choose_swapchain_extent(capabilities: vk::SurfaceCapabilitiesKHR, window_width: u32, window_height: u32) -> vk::Extent2D {
		let mut width = window_width;
		let mut height = window_height;

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

	//Create image views for the swapchain images
	//Input a vec of images from swapchain_req.swapchain_images
	fn create_image_views(device: &ash::Device, surface_format: vk::SurfaceFormatKHR, images: &Vec<vk::Image>) -> Vec<vk::ImageView> {
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

	//Create a render pass for the pipeline
	//Decribes framebuffer attachments to be used when rendering
	//Dynamic rendering ("VK_KHR_dynamic_rendering") would make it so this isn't really necessary (makes each render pass just one subpass). Subpasses are really only important for phone GPUs (tiled GPUs)
	fn create_render_pass(device: &ash::Device, surface_format: vk::Format) -> vk::RenderPass {
		//First create attachment description
		//There's also an AttachmentDescription2, but it only really adds s_type and p_next
		let color_attachment = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: surface_format, //Will take this from swapchain so they're compatible
			samples: vk::SampleCountFlags::TYPE_1, //Samples per pixel if msaa is being used
			load_op: vk::AttachmentLoadOp::CLEAR, //What to do with the attachment at the beginning of the first subpass (color/depth components)
			store_op: vk::AttachmentStoreOp::STORE, //What to do with the attachment at the end of the last subpass (color/depth components)
			stencil_load_op: vk::AttachmentLoadOp::DONT_CARE, //and load behavior for the stencil component
			stencil_store_op: vk::AttachmentStoreOp::DONT_CARE, //and store behavior for the stencil component
			initial_layout: vk::ImageLayout::UNDEFINED, //Input image layout - "UNDEFINED" usually means the image was just created
			final_layout: vk::ImageLayout::PRESENT_SRC_KHR //Output image layout - we want it to go straight to the swapchain
		};

		//Subpasses will reference the attachments, need to set up the attachment references
		let color_attachment_ref = vk::AttachmentReference {
			attachment: 0, //Index of attachment to use in RenderPassCreateInfo
			layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, //Image layout to use during the subpass
		};

		//Subpass description
		//There's also a "SubpassDescription2," which adds a view mask for multiview - don't really need it
		let subpass = vk::SubpassDescription {
			flags: vk::SubpassDescriptionFlags::empty(),
			pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS, //Pipeline type supported for the subpass, graphics/compute
			input_attachment_count: 0, //Attachments read from shader
			p_input_attachments: ptr::null(),
			color_attachment_count: 1,
			p_color_attachments: &color_attachment_ref,
			p_resolve_attachments: ptr::null(), //Attachments used for multisampling
			p_depth_stencil_attachment: ptr::null(), //Attachment for depth/stencil data
			preserve_attachment_count: 0, //Attachments that aren't used by this subpass, but need to be preserved
			p_preserve_attachments: ptr::null(),
			..Default::default()
		};

		//Make a subpass dependency for synchronization stuff - image layout transition in render pass must wait until 
		//The first implicit subpass has an implicit subpass dependency already, but that dependency is at the top of the pipe
		//Need to make sure render passes don't begin until the image is available, but without this, there's nothing stopping a subpass from executing at the top of the pipe
		//So this has the color output stage of subpass 0 (the dependent subpass) wait until the color output + write from the dependency (the first implicit subpass), which won't happen while the semaphore is a thing
		let subpass_dependencies = [vk::SubpassDependency {
			src_subpass: vk::SUBPASS_EXTERNAL, //Dependency - "SUBPASS_EXTERNAL" refers to operations that happen before the render pass
			dst_subpass: 0, //Subpass index of the dependent subpass
			src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, //Stage of the src subpass for the dst subpass to wait for - once the src subpass gets here, the dst subpass is allowed to go ahead
			src_access_mask: vk::AccessFlags::empty(), //We're not waiting on any memory dependency, we just need to know that the color output stage (and thus the semaphore + swapchain image acquisition) has executed
			dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, //Operations that should wait (writing of the color attachment) - so the render pass is allowed to execute up to this point
			dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE, //Dst subpass waits on writing to the color attachment
			dependency_flags: vk::DependencyFlags::empty(),
		}];

		//Render pass creation info
		//There's also a "RenderPassCreationInfo2" that adds a mask suggesting views that should be rendered concurrently, not necessary
		let render_pass_info = vk::RenderPassCreateInfo {
			s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::RenderPassCreateFlags::empty(),
			attachment_count: 1,
			p_attachments: &color_attachment,
			subpass_count: 1,
			p_subpasses: &subpass,
			dependency_count: subpass_dependencies.len() as u32, //Memory dependencies between subpasses
			p_dependencies: subpass_dependencies.as_ptr(),
			..Default::default()
		};

		//Create and return the render pass
		unsafe { device.create_render_pass(&render_pass_info, None).expect("Failed to create render pass") }
	}

	//Create the pipeline
	//Most of the pipeline must be baked. Can configure certain things to be dynamic with "PipelineDynamicStateCreateInfo"
	//Dynamic states will be recorded into the command buffer after the pipeline is bound
	fn create_pipeline(device: &ash::Device, render_pass: vk::RenderPass, swapchain_extent: vk::Extent2D) -> (vk::Pipeline, vk::PipelineLayout) {
		//Start with the programmable pipeline stages
		//Read the spirv files for the vertex/fragment shaders
		//Shader modules should be destroyed after pipeline creation
		let fragment_shader_code = fragment_shader();
		let vertex_shader_code = vertex_shader();

		//Create the shader modules from those files
		let fragment_shader_module = VulkanApp::create_shader_module(device, fragment_shader_code);
		let vertex_shader_module = VulkanApp::create_shader_module(device, vertex_shader_code);
		
		//Define the shader entry point - the function of the shader that will run. We want "main" to run (the only one in there at the moment)
		//Have to define here because rust doesn't like the lifetime of the pointer dying so quickly
		let shader_entry_point = CString::new("main").unwrap();

		//Pipeline fragment shader stage
		let fragment_stage_info = vk::PipelineShaderStageCreateInfo {
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
		let vertex_stage_info = vk::PipelineShaderStageCreateInfo {
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
		let shader_stages = [fragment_stage_info, vertex_stage_info];
		
		//Now it's time for all the fixed function pipeline stages
		//Start by making a vec to track any states we want to make dynamic. Will just push flags in there as we go
		let mut dynamic_states = vec![];

		//Get binding + attribute descriptions for the vertex input stage
		let binding_descriptions = Vertex::get_binding_descriptions();
		let attribute_descriptions = Vertex::get_attribute_descriptions();

		//Vertex input create info. This has to do with has vertices are passed to the vertex shader
		let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineVertexInputStateCreateFlags::empty(),
			vertex_attribute_description_count: attribute_descriptions.len() as u32,
			p_vertex_attribute_descriptions: attribute_descriptions.as_ptr(),
			vertex_binding_description_count: binding_descriptions.len() as u32,
			p_vertex_binding_descriptions: binding_descriptions.as_ptr(),
			..Default::default()
		};

		//Input assembly create info. Describes primitive topology
		let input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
			topology: vk::PrimitiveTopology::TRIANGLE_LIST, //Defines every 3 vertices as a triangle primitive, no overlap
			primitive_restart_enable: vk::FALSE, //Will allow a "restart" vertex index value that can break shit up. Irrelevant for triangle list mode
			..Default::default()
		};
		
		//Viewport and scissor - region of the framebuffer that will be rendered to. We want to make these equal to our WINDOW extent at the current frame
		//For that reason, must be dynamic
		let viewport_state_info = vk::PipelineViewportStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineViewportStateCreateFlags::empty(),
			viewport_count: 1, //Must specify count even if dynamic
			p_viewports: ptr::null(), //Ignored if dynamic state
			scissor_count: 1, //Must specify count even if dynamic
			p_scissors: ptr::null(), //Ignored if dynamic state
			..Default::default()
		};

		//Viewport and scissors will be dynamic oooh ahhhh
		dynamic_states.push(vk::DynamicState::VIEWPORT);
		dynamic_states.push(vk::DynamicState::SCISSOR);

		//Rasterization stage configuration
		let rasterization_state_info = vk::PipelineRasterizationStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineRasterizationStateCreateFlags::empty(),
			depth_clamp_enable: vk::FALSE, //Clamps to z_near and z_far planes instead of discarding values past them if true (requires an extension)
			rasterizer_discard_enable: vk::FALSE, //Discards triangles before rasterizing. Disables output to framebuffer basically
			polygon_mode: vk::PolygonMode::FILL, //Can do wireframe or whatever
			line_width: 1.0, //Any line thicker than 1.0 will require GPU feature
			cull_mode: vk::CullModeFlags::BACK, //Cull the back facing triangles only
			front_face: vk::FrontFace::COUNTER_CLOCKWISE, //Defines triangle winding convention used for face culling
			depth_bias_enable: vk::FALSE, //Bias on all the depth values. I guess it can be used for shadow maps or something
			depth_bias_constant_factor: 0.0,
			depth_bias_clamp: 0.0,
			depth_bias_slope_factor: 0.0,
			..Default::default()
		};

		//MSAA disabled for now - requires a gpu feature
		//Good for forward rendering, not so much for deferred (needs to know vertex edges. If lighting is deferred, it won't know anything about vertices)
		let ms_state_info = vk::PipelineMultisampleStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineMultisampleStateCreateFlags::empty(),
			rasterization_samples: vk::SampleCountFlags::TYPE_1, //Number of samples per pixel
			sample_shading_enable: vk::FALSE, //Sample shading - this can force a certain number of samples for EVERY pixel, rather than just edges (enables fsaa/ssaa)
			min_sample_shading: 0.0, //If sample shading is enabled, can force a fraction of the "rasterization_samples" number to be taken, or just go 1.0 for all of them
			p_sample_mask: ptr::null(),
			alpha_to_one_enable: vk::FALSE, //If enabled: will replace fragment's alpha with msaa coverage
			alpha_to_coverage_enable: vk::FALSE, //If enabled: will generate a temp alpha for msaa coverage and combine it with the fragment's aplpha
			..Default::default()
		};

		//Configures depth/stencil tests if using depth/stencil buffer
		//Not using for now
		//First have to comfigure stencil state - right now, not using the stencil buffer - will always keep everything
		//Can setup dynamic enabling/disabling of the stencil test if need be
		let stencil_state = vk::StencilOpState {
			fail_op: vk::StencilOp::KEEP, //What to do to samples that fail stencil test
			pass_op: vk::StencilOp::KEEP, //What to do with samples that pass the stencil and depth tests
			depth_fail_op: vk::StencilOp::KEEP, //What to do with samples that pass the stencil test but fail depth test
			compare_op: vk::CompareOp::ALWAYS, //Comparison operator to use for stencil test
			compare_mask: 0, //Can set which bits of the stencil value to check
			write_mask: 0, //Can set bits of stencil values in the stencil framebuffer attachment that get updated by the stencil test
			reference: 0, //Value to test the stencil value against
		};

		let depth_stencil_state_info = vk::PipelineDepthStencilStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
			depth_test_enable: vk::FALSE, //Enables depth testing - compares new fragments to depth buffer
			depth_write_enable: vk::FALSE, //Enables whether depth attachment is written to if the comparison comes back as "true" during depth test (sets to sample's depth if so)
			depth_compare_op: vk::CompareOp::LESS_OR_EQUAL, //What operator to use for depth comparison (lower depth is closer by convention)
			depth_bounds_test_enable: vk::FALSE, //This and the two bounds let you discard things in a certain depth range. Don't really need it
			min_depth_bounds: 0.0,
			max_depth_bounds: 1.0,
			stencil_test_enable: vk::FALSE, //Enable stencil test. Will have to make sure the depth/stencil image has a stencil component if using this
			front: stencil_state, //For front facing triangles
			back: stencil_state, //For back facing triangles
			..Default::default()
		};

		//Color blending - controls how fragment shader's returned color mixes with the color already in the framebuffer
		//Need the attachment states first
		//Disable blending for now - framebuffer will just take new color
		let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
			blend_enable: vk::FALSE,
			src_color_blend_factor: vk::BlendFactor::ONE, //What the source color (new color from fragment buffer) is multiplied by for blending
			dst_color_blend_factor: vk::BlendFactor::ONE, //What the destination color (old color in framebuffer) is multiplied by before blending
			color_blend_op: vk::BlendOp::ADD, //The operation used to blend the two colors
			src_alpha_blend_factor: vk::BlendFactor::ONE, //Same thing is done again for alpha
			dst_alpha_blend_factor: vk::BlendFactor::ONE,
			alpha_blend_op: vk::BlendOp::ADD,
			color_write_mask: vk::ColorComponentFlags::RGBA //Enables/disables any of the rgba components for writing
		}];

		let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineColorBlendStateCreateFlags::empty(),
			logic_op_enable: vk::FALSE, //If enabled, this will ignore the "color_blend_attachment" and just apply the bitwise operation in "logic_op" for blending instead
			logic_op: vk::LogicOp::COPY,
			attachment_count: color_blend_attachments.len() as u32,
			p_attachments: color_blend_attachments.as_ptr(),
			blend_constants: [0.0, 0.0, 0.0, 0.0], //Will be used if "BlendFactor" needs constants for an operation
			..Default::default()
		};

		//Dynamic state create info
		let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineDynamicStateCreateFlags::empty(),
			dynamic_state_count: dynamic_states.len() as u32,
			p_dynamic_states: dynamic_states.as_ptr(),
			..Default::default()
		};
		
		//Setup shader push constants to be used in pipeline layouts
		//Push constants are mega-small (~128 bytes at minimum, so 2 glam::f32::Mat4s), but are very fast, and are updated via commands rather than memory/copy commands
		//I'm using them over a uniform buffer because I'm recording commands each frame anyway, so these will slot in nicely
		//Just want to push view + projection matrix for now - will only be used in the vertex bit
		let push_constant_ranges = [vk::PushConstantRange {		
			stage_flags: vk::ShaderStageFlags::VERTEX,
			offset: 0,
			size: core::mem::size_of::<glam::f32::Mat4>() as u32,
		}];

		//Pipeline layout - describes resources that can be accessed by a pipeline (descriptor sets)
		//Use push constants for transformation matrices rather than uniform buffers - recording 
		let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
			s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineLayoutCreateFlags::empty(),
			set_layout_count: 0, //Number of descriptor sets in pipeline layout
			p_set_layouts: ptr::null(), //Pointer to descriptor set layouts
			push_constant_range_count: push_constant_ranges.len() as u32, //Number of push constants in pipeline layout
			p_push_constant_ranges: push_constant_ranges.as_ptr(), //Pointer to push constants layouts
			..Default::default()
		};

		//Create the pipeline layout
		let pipeline_layout = unsafe { device.create_pipeline_layout(&pipeline_layout_info, None).expect("Failed to create pipeline layout") };

		//Pipeline creation info
		//Can use pipeline with subpasses other than the one defined here, but it must be a compatible render pass (same formats/sample count for all relevant attachments - it's all in spec if need be)
		let pipeline_info = vk::GraphicsPipelineCreateInfo {
			s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineCreateFlags::empty(),
			stage_count: shader_stages.len() as u32,
			p_stages: shader_stages.as_ptr(),
			p_vertex_input_state: &vertex_input_state_info,
			p_input_assembly_state: &input_assembly_state_info,
			p_tessellation_state: ptr::null(),
			p_viewport_state: &viewport_state_info,
			p_rasterization_state: &rasterization_state_info,
			p_multisample_state: &ms_state_info,
			p_depth_stencil_state: &depth_stencil_state_info,
			p_color_blend_state: &color_blend_state_info,
			p_dynamic_state: &dynamic_state_info,
			layout: pipeline_layout, //Defined above
			render_pass, //Passed into "create_pipeline"
			subpass: 0, //Subpass index in render pass that this pipeline will be used
			base_pipeline_handle: vk::Pipeline::null(), //If creating a derivative pipeline of another pipeline, use these. Also will have to set the appropriate flag above to enable these
			base_pipeline_index: -1, //-1 if no parent pipeline
			..Default::default()
		};

		//Pipeline creation function can create multiple pipelines at once. Setup the array here
		let pipeline_infos = [pipeline_info];

		//Create the pipeline
		//Pipeline cache allows for reuse of pipeline creation details, can speed creation of pipelines later. "Leave as vk::PipelineCache::null()" to not use it
		let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_infos, None).expect("Failed to create graphics pipeline(s)") };

		//Destroy the shader modules now, since they won't be needed after the pipeline gets created
		unsafe {
			device.destroy_shader_module(vertex_shader_module, None);
			device.destroy_shader_module(fragment_shader_module, None);
		}

		//Return the pipeline and pipeline layout
		//Just return the one pipeline being created
		(pipelines[0], pipeline_layout)
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

	//Creates framebuffers to hold attachments needed for the render pass
	//Iterate through image views, create framebuffer for each one
	fn create_framebuffers(device: &ash::Device, render_pass: vk::RenderPass, image_views: &Vec<vk::ImageView>, swapchain_extent: vk::Extent2D) -> Vec<vk::Framebuffer> {
		let mut framebuffers = vec![];
		
		//Loop through the swapchain image views, get a framebuffer for each one
		//Need a framebuffer for each image view to write to whenever the swapchain does its whole swap thing
		for &image_view in image_views {
			let attachments = [image_view];
			let framebuffer_info = vk::FramebufferCreateInfo {
				s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::FramebufferCreateFlags::empty(),
				render_pass, //Render pass used for framebuffer compatibility
				attachment_count: 1, //Just the color attachment for now
				p_attachments: attachments.as_ptr(),
				width: swapchain_extent.width, //Framebuffer will have the same extent as the swap chain image view (which has the same extent as the swap chain images)
				height: swapchain_extent.height,
				layers: 1,
				..Default::default()
			};

			//Create a framebuffer and add it to the vec
			let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None).expect("Failed to create framebuffer") };
			framebuffers.push(framebuffer);
		}
		//Return the framebuffers vec
		framebuffers
	}

	//Creates a command pool - used to manage memory for command buffers
	fn create_command_pools(device: &ash::Device, queue_family_indices: &QueueFamilyIndices) -> (vk::CommandPool, vk::CommandPool) {
		let command_pool_info = vk::CommandPoolCreateInfo {
			s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER, //Want to be able to reset the command buffers individually (will happen every frame in this app with "vkResetCommandBuffer")
			queue_family_index: queue_family_indices.graphics_family.unwrap(), //Want to submit commands for drawing to a command buffer on the graphics queue
			..Default::default()
		};

		//Create the command pool
		let command_pool = unsafe { device.create_command_pool(&command_pool_info, None).expect("Failed to create command pool") };

		//Create a second command pool with the transient bit set - will only create short lived command buffers
		//Will be used for transfer operations, but uses the graphics queue family - graphics family is guaranteed to support transfer by spec
		//Transfer is really only relevant when working with multiple CPU threads
		let command_pool_info = vk::CommandPoolCreateInfo {
			s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::CommandPoolCreateFlags::TRANSIENT, //Command buffers will be short lived. Driver might optimize for that if it sees this flag
			queue_family_index: queue_family_indices.graphics_family.unwrap(), //This is going to be used for 
			..Default::default()
		};

		//Create the command pool
		let command_pool_short = unsafe { device.create_command_pool(&command_pool_info, None).expect("Failed to create command pool") };

		(command_pool, command_pool_short)
	}

	//Allocates and creates command buffers for commands to be submitted to
	//Currently creating one command buffer for the one frame in flight
	//Then, command buffers can be reused + rerecorded during frame draw
	fn create_command_buffer(device: &ash::Device, command_pool: vk::CommandPool) -> Vec<vk::CommandBuffer> {
		//As long as CPU/GPU are going fast enough, don't need multiple frames in flight
		//Frames in flight are only there to give CPU something to do while the GPU chugs away, but the CPU will get farther ahead (more input lag)
		let command_buffer_info = vk::CommandBufferAllocateInfo {
			s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
			p_next: ptr::null(),
			command_pool, //Command pool from which command buffer is allocated
			level: vk::CommandBufferLevel::PRIMARY, //Primary or secondary. Primary command buffers can execute secondary command buffers, kinda like executing a function
			command_buffer_count: 1, //Number of command buffers to allocate. If doing multiple frames in flight, must have one for each framebuffer (one for each swapchain image)
			..Default::default()
		};

		//Allocate and return the command buffer
		unsafe { device.allocate_command_buffers(&command_buffer_info).expect("Failed to allocate command buffers") }
	}

	//Creates a vertex buffer - will hold vertex data
	//Uses a staging buffer that is host visible and host coherent. Then, will transfer that to device local memory (faster)
	fn create_vertex_buffer(instance: &ash::Instance, device: &ash::Device, physical_device: vk::PhysicalDevice, command_pool: vk::CommandPool, submit_queue: vk::Queue) -> (vk::Buffer, vk::DeviceMemory) {
		//Setup size + usage for the vertex buffer
		let buffer_size = core::mem::size_of_val(&TEST_TRIANGLE_VERTICES) as u64;
		let staging_buffer_usage = vk::BufferUsageFlags::TRANSFER_SRC; //Staging buffer will end up transferring to the vertex buffer
		//If host coherent, "vkFlushMappedMemoryRanges" and "vkInvalidateMappedMemoryRanges" aren't needed during memory mapping, but it's slower
		let staging_required_memory_properties = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
		
		//Create the staging buffer
		let (staging_buffer, staging_buffer_memory) = create_buffer(instance, device, physical_device, buffer_size, staging_buffer_usage, staging_required_memory_properties);

		//Have to fill the vertex buffer - map buffer memory into CPU accessible memory
		//This gives a pointer to a region of mappable memory
		let p_mappable = unsafe { device.map_memory(staging_buffer_memory, 0, buffer_size, vk::MemoryMapFlags::empty()).expect("Failed to map device memory") as *mut Vertex};
		//Copy the data into that mappable memory - rust equivalent of "memcpy"
		unsafe { ptr::copy_nonoverlapping(TEST_TRIANGLE_VERTICES.as_ptr(), p_mappable, TEST_TRIANGLE_VERTICES.len()) };
		//Unmap the memory. Typically we can't guarantee the order, and would have to use "vkFlushMappedMemoryRanges" and "vkInvalidateMappedMemoryRanges"
		//The memory was chosen to be host coherent with "vk::MemoryPropertyFlags::HOST_COHERENT" so those aren't needed
		unsafe { device.unmap_memory(staging_buffer_memory) };

		//Now create the vertex buffer
		let vertex_buffer_usage = vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER; //Want both the transfer destination bit the and vertex buffer bit
		let vertex_required_memory_properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;

		let (vertex_buffer, vertex_buffer_memory) = create_buffer(instance, device, physical_device, buffer_size, vertex_buffer_usage, vertex_required_memory_properties);

		//Copy the staging buffer into the vertex buffer
		//Pass in graphics queue, since that's required to support transfer by spec. Could find a separate queue for transfer operations, but this is really only a concern when multithreading transfers
		copy_buffer(device, command_pool, submit_queue, staging_buffer, vertex_buffer, buffer_size);

		//Can get rid of the staging buffers now
		unsafe { device.destroy_buffer(staging_buffer, None) };
		unsafe { device.free_memory(staging_buffer_memory, None) };

		//Return the vertex buffer as well as its memory to be freed later
		(vertex_buffer, vertex_buffer_memory)
	}

	//Create synchronization objects to deal with frames in flight + swapchain sync stuff
	fn create_sync_objects(device: &ash::Device) -> (vk::Semaphore, vk::Semaphore, vk::Fence) {
		let semaphore_info = vk::SemaphoreCreateInfo {
			s_type: vk::StructureType::SEMAPHORE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::SemaphoreCreateFlags::empty(),
			..Default::default()
		};

		let fence_info = vk::FenceCreateInfo {
			s_type: vk::StructureType::FENCE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::FenceCreateFlags::SIGNALED, //Create the fence signaled to show that it's free
			..Default::default()
		};

		//Create semaphores + fences
		let image_available_semaphore = unsafe { device.create_semaphore(&semaphore_info, None).expect("Failed to create semaphore") };
		let render_finished_semaphore = unsafe { device.create_semaphore(&semaphore_info, None).expect("Failed to create semaphore") };
		let in_flight_fence = unsafe { device.create_fence(&fence_info, None).expect("Failed to create fence") };

		//Return the semaphores + fences in a tuple
		(image_available_semaphore, render_finished_semaphore, in_flight_fence)
	}

	//A little note - all of the above functions didn't use "self" because they were to be called in "init_vulkan." These next ones aren't, and pertain to when the event loop is running

	//Draw a frame to the surface
	//Because of how this is synchronized, this will draw start drawing the frame, then get the swapchain image, then finish drawing the frame to that swapchain image
	//Need to pass in the window to get width/height, need to pass scene info
	pub fn draw_frame(&self, window: &Window, scene: &Scene) {
		//If the window is size 0, don't even deal with it
		//Running into too many problems with keeping the command buffer extent + framebuffer extent + swapchain extent gucchi. Maybe a later solution will be
		if window.inner_size().width == 0 || window.inner_size().height == 0 {
			return
		}

		//Just have on frame in flight
		let current_frame_fence_array = [self.in_flight_fence];

		//Only waiting for the fence for the current frame. With one frame in flight, this is all there is
		unsafe { self.device.wait_for_fences(&current_frame_fence_array, true, std::u64::MAX).expect("Failed to wait for fence") }; //No timeout, set as the max u64

		//Acquire next image from swapchain
		//The command buffer will be queued on this image index, so will need to use the appropriate command buffer
		let (image_index, is_suboptimal) = match unsafe { self.swapchain_loader.acquire_next_image(self.swapchain, std::u64::MAX, self.image_available_semaphore, vk::Fence::null())} {
			Ok((image_index, is_suboptimal)) => (image_index, is_suboptimal),
			Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => return, //If the swapchain image is out of date, return out of this function without drawing the frame
			_ => panic!("Failed to acquire next swapchain image")
		};

		//Need the window's width and height to record the command buffer
		unsafe { self.device.reset_command_buffer(self.command_buffers[0], vk::CommandBufferResetFlags::empty()).expect("Failed to reset command buffer"); } //Reset
		self.record_command_buffer(window, scene, image_index as usize); //Record into the command buffers
		
		//After waiting, have to reset the fence
		//Delay resetting fence until we know acquire_next_image succeeded, in case of any weird behavior with resizing
		unsafe {self.device.reset_fences(&current_frame_fence_array).expect("Failed to reset fences") };

		//Setup semaphores into arrays to deal with queue submission
		//Want to wait at the color attachment output stage - don't want to output any colors until the image to write to becomes available
		//This allows vertex shader to be run while still waiting for a swapchain image
		let wait_available_array = [self.image_available_semaphore];
		let wait_pipeline_stage = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
		//This will be the semaphore to signal when the command buffers finish executing
		let signal_finished_array = [self.render_finished_semaphore];

		//Info for command buffer to be submitted to the queue
		let submit_infos = [vk::SubmitInfo {
			s_type: vk::StructureType::SUBMIT_INFO,
			p_next: ptr::null(),
			wait_semaphore_count: wait_available_array.len() as u32, //Number of semaphores to wait at
			p_wait_semaphores: wait_available_array.as_ptr(), //Array of semaphores to wait at
			p_wait_dst_stage_mask: wait_pipeline_stage.as_ptr(), //Array of pipeline stages that get waited at. These correspond to the p_wait_semaphores
			command_buffer_count: 1, //Number of command buffers
			p_command_buffers: &self.command_buffers[0], //Which command buffer to queue - just have the one frame in flight
			signal_semaphore_count: signal_finished_array.len() as u32,
			p_signal_semaphores: signal_finished_array.as_ptr(), //Semaphore to signal when the command buffers finish
			..Default::default()
		}];

		//Submit command buffer to queue
		//Signals fence once the command buffers complete execution - can then reuse the command buffer
		unsafe {self.device.queue_submit(self.graphics_queue, &submit_infos, self.in_flight_fence).expect("Failed to submit command buffer to queue") };

		//Need an array of the swapchains for the present info
		let swapchains_array = [self.swapchain];

		//Presentation info with semaphores and swapchains and stuff
		let present_info = vk::PresentInfoKHR {
			s_type: vk::StructureType::PRESENT_INFO_KHR,
			p_next: ptr::null(),
			wait_semaphore_count: signal_finished_array.len() as u32, //Number of semaphores to wait at
			p_wait_semaphores: signal_finished_array.as_ptr(), //Array of semaphores to wait at
			swapchain_count: swapchains_array.len() as u32,
			p_swapchains: swapchains_array.as_ptr(),
			p_image_indices: &image_index,
			p_results: ptr::null_mut(), //Can use this to check if presentation was successful for an array of swapchains
			..Default::default()
		};

		//Queue an image for presentation
		//If "ERROR_OUT_OF_DATE_KHR" error happens, it means the apphandler didn't handle the resize, so one of the window dimensions must be zero
		match unsafe { self.swapchain_loader.queue_present(self.present_queue, &present_info) } {
			Ok(_) => (),
			Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => (), //If the swapchain image is out of date, return out of this function without drawing the frame
			_ => panic!("Failed to execute queue present")
		}
	}

	//Will record during frame draw
	//When only frame is in flight, it's probably faster to reuse one command buffer and just rerecord it for the appropiate swapchain image
	//Frames in flight are only there to give CPU something to do while GPU chugs away, but they increase lag by letting the CPU game physics go farther ahead than the rendering
	fn record_command_buffer(&self, window: &Window, scene: &Scene, image_index: usize) {
		//First, setup everything needed with in VulkanApp (there's a bunch)
		let device = &self.device;
		let command_buffer = self.command_buffers[0];
		let render_pass = self.render_pass;
		let pipeline = self.pipeline;
		let pipeline_layout = self.pipeline_layout;
		let framebuffer = self.swapchain_framebuffers[image_index];
		let vertex_buffer = self.vertex_buffer;
		let window_width = window.inner_size().width;
		let window_height = window.inner_size().height;

		//Start with the command buffer begin info
		let command_buffer_begin_info = vk::CommandBufferBeginInfo {
			s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
			p_next: ptr::null(),
			flags: vk::CommandBufferUsageFlags::empty(), //Flags for rerecording, resubmitting, and if it's a secondary command buffer. None needed here
			p_inheritance_info: ptr::null(), //Inheritance info for secondary command buffers
			..Default::default()
		};

		//Begin recording to the command buffer
		//Remember - the commands submitted to the buffer will NOT necessarily go in order
		unsafe { device.begin_command_buffer(command_buffer, &command_buffer_begin_info).expect("Failed to begin recording to command buffer") };

		//Set the values to clear to, this will be passed to "render_pass_begin_info"
		//An array containing clear values for each framebufffer attachment that has a loap_op (as defined in the render pass) with clearing
		//This is a rust union, so it's defined using one field
		let clear_values = [vk::ClearValue {
			color: vk::ClearColorValue {float32: [0.0, 0.0, 0.0, 1.0]}, //Black at 100% opacity
		}];

		//Render pass begin info
		let render_pass_begin_info = vk::RenderPassBeginInfo {
			s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
			p_next: ptr::null(),
			render_pass, //The render pass to begin an instance of
			framebuffer, //The framebuffer containing the attachments to use in the render pass
			render_area: vk::Rect2D { //Render area being affected by the render pass instance
				offset: vk::Offset2D {x: 0, y: 0},
				extent: vk::Extent2D {width: window_width, height: window_height}, //Will be different if window is resized
			},
			clear_value_count: clear_values.len() as u32,
			p_clear_values: clear_values.as_ptr(),
			..Default::default()
		};

		//Command to begin the render pass
		//There's a begin_render_pass2, but it only adds a s_type and p_next to the SubpassContents
		unsafe { device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE) }; //Inline: subpass commands will be in primary command buffer, no secondary command buffers

		//Bind the pipeline to the render pass
		//Pipeline bind point is graphics - not using compute
		//Dynamic states would be set here, if they were set up in "create_pipeline" fn
		unsafe { device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline) }; //Specified as graphics pipeline, same as specification in render pass subpass

		//Bind the vertex buffer
		let vertex_buffers = [vertex_buffer];
		let offsets = [0];
		unsafe { device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets) };

		//Calculate the matrix to push to the shaders
		//Need to make sure alignment rules are held to - since this is just a single Mat4 of 128 bytes
		let render_matrix_bytes = scene.render_matrix_bytes();
		//Push the matrix as a push constant
		unsafe { device.cmd_push_constants(command_buffer, pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, &render_matrix_bytes) };

		//Setup the viewport
		let viewports = [vk::Viewport {
			x: 0.0, //Top left
			y: 0.0, //Top left
			width: window_width as f32, //Window width
			height: window_height as f32, //Window height
			min_depth: 0.0, //Just keep standard depths
			max_depth: 1.0
		}];
		//Set viewport
		unsafe { device.cmd_set_viewport(command_buffer, 0, &viewports); }

		//Setup the scissors to be used with the viewport 
		let scissors = [vk::Rect2D {
			offset: vk::Offset2D {x: 0, y: 0}, //No offset
			extent: vk::Extent2D {width: window_width, height: window_height} //Will change based on window size
		}];
		//Set the scissors
		unsafe { device.cmd_set_scissor(command_buffer, 0, &scissors); }

		//Draw command
		unsafe { device.cmd_draw(command_buffer, TEST_TRIANGLE_VERTICES.len() as u32, 1, 0, 0) }; //Specify number of vertices, number of instances, vertex offset, instance offset

		//Command to end the render pass
		unsafe { device.cmd_end_render_pass(command_buffer)};

		//End command buffer recording
		unsafe { device.end_command_buffer(command_buffer).expect("Failed to record to command buffer") };
	}


	//Wait until idle - will need to call from apphandler when close is requested to make sure drawing/presenting options aren't happening
	//This should ONLY be used when things need to be destroyed
	pub fn wait_idle(&self) {
		unsafe { self.device.device_wait_idle().expect("Failed to wait until device idle") }
	}

	//Function to call on a window resize event
	//Would also want to do it on a "ERROR_OUT_OF_DATE_KHR" error from "acquire_next_image," but then "draw_frame" would require the window as an argument and would be mutable - just not necessary yet
	//Gonna have to recreate everything that depends on swapchain/swapchain extents
	pub fn recreate_swapchain(&mut self, window: &Window) {
		//Wait until program isn't doing anything to destroy/recreate
		unsafe { self.device.device_wait_idle().expect("Failed to wait until device idle") }

		//Destroy the stuff that'll be replaced
		//Need to free the command buffers - not destroying the command pool, so need to go directly to command buffers for this
		unsafe {
			for framebuffer in &self.swapchain_framebuffers {
				self.device.destroy_framebuffer(*framebuffer, None);
			}

			for swapchain_image_view in &self.swapchain_image_views {
				self.device.destroy_image_view(*swapchain_image_view, None);
			}
			self.swapchain_loader.destroy_swapchain(self.swapchain, None);

			self.surface_loader.destroy_surface(self.surface, None);
		}

		//Get window width + height being rendered to
		//Will need when creating swapchains
		let window_width = window.inner_size().width;
		let window_height = window.inner_size().height;

		//To recreate swapchain, need new surface/surface loader
		//Need to pass in the new window dimensions
		let surface_req = VulkanApp::create_surface(&self.entry, &self.instance, window);
		//Also need queue family indices
		let queue_family_indices = QueueFamilyIndices::find_queue_families(&self.instance, self.physical_device, &surface_req);

		//Now, recreate swapchain based on new surface
		let swapchain_req = VulkanApp::create_swapchain(&self.instance, &self.device, self.physical_device, &surface_req, &queue_family_indices, window_width, window_height);
		//Recreate the image views
		let swapchain_image_views = VulkanApp::create_image_views(&self.device, swapchain_req.swapchain_format, &swapchain_req.swapchain_images);
		//Recreate the framebuffers that contain the image views for the swapchain images as attachments
		let swapchain_framebuffers = VulkanApp::create_framebuffers(&self.device, self.render_pass, &swapchain_image_views, swapchain_req.swapchain_extent);

		//NOT going to recreate the render pass. Theoretically, this might cause problems is window is moved to like an HDR monitor. WHATEVER!
		//Also not recreating pipeline
		//And not recreating command buffer, since it's recorded into during frame draw

		//Update everything in VulkanApp that needs to be updated
		self.surface = surface_req.surface;
		self.surface_loader = surface_req.surface_loader;

		self.swapchain = swapchain_req.swapchain;
		self.swapchain_loader = swapchain_req.swapchain_loader;
		self.swapchain_image_views = swapchain_image_views;
		self.swapchain_framebuffers = swapchain_framebuffers;
	}
}

//Have to destroy anything that was explicitly created
impl Drop for VulkanApp {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_semaphore(self.image_available_semaphore, None);
			self.device.destroy_semaphore(self.render_finished_semaphore, None);
			self.device.destroy_fence(self.in_flight_fence, None);

			self.device.destroy_buffer(self.vertex_buffer, None);
			self.device.free_memory(self.vertex_buffer_memory, None);

			self.device.destroy_command_pool(self.command_pool, None);
			self.device.destroy_command_pool(self.command_pool_short, None);
			for framebuffer in &self.swapchain_framebuffers {
				self.device.destroy_framebuffer(*framebuffer, None);
			}

			self.device.destroy_pipeline(self.pipeline, None);
			self.device.destroy_pipeline_layout(self.pipeline_layout, None);

			self.device.destroy_render_pass(self.render_pass, None);

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