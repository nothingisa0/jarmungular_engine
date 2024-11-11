use ash::vk;
use std::ptr;

//Creates a buffer
//As a note - vulkan doesn't like allocating lots of memory separately, it prefers to do it in a big chunk - there's a limit to the number of times memory can be allocated
//Generally, a few big chunks of memory will be allocated and everything will be copied over together. This makes fragmentation an issue to think about
pub fn create_buffer(instance: &ash::Instance, device: &ash::Device, physical_device: vk::PhysicalDevice, size: vk::DeviceSize, usage: vk::BufferUsageFlags, required_memory_properties:  vk::MemoryPropertyFlags) -> (vk::Buffer, vk::DeviceMemory) {
	//Buffer creation info
	let buffer_info = vk::BufferCreateInfo {
		s_type: vk::StructureType::BUFFER_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::BufferCreateFlags::empty(), //There's some flags to make it a sparce resource - don't need them
		size, //Size of all the data
		usage, //Buffer usage
		sharing_mode: vk::SharingMode::EXCLUSIVE, //Doesn't need to be shared - will only be used by the graphics queue
		queue_family_index_count: 0, //Ignored if sharing mode is exclusive
		p_queue_family_indices: ptr::null(), //Ignored if sharing mode is exclusive
		..Default::default()
	};

	//Create the buffer
	let buffer = unsafe { device.create_buffer(&buffer_info, None).expect("Failed to create vertex buffer") };

	//Get that buffer's memory requirements - required size may differ from the size specified during buffer creation
	let buffer_memory_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

	//Need to find the right type of GPU memory to use query the GPU using "find_memory_type_index"
	let memory_type_index = find_memory_type_index(instance, physical_device, buffer_memory_requirements.memory_type_bits, required_memory_properties);

	//After getting the appropriate memory type index, can fill out memory allocation info
	let memory_allocate_info = vk::MemoryAllocateInfo {
		s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
		p_next: ptr::null(),
		allocation_size: buffer_memory_requirements.size,
		memory_type_index,
		..Default::default()
	};

	//Allocate the memory
	let buffer_memory = unsafe { device.allocate_memory(&memory_allocate_info, None).expect("Failed to allocate device memory") };

	//Associate the allocated memory to the buffer by binding it - no offset, since the memory is specifically allocated for this buffer
	unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0).expect("Failed to bind buffer memory") };

	(buffer, buffer_memory)
}

//Can copy a buffer in host visible memory to a buffer in device local memory
//This function won't check if the supplied queue has transfer capabilities, but that should be ensured first
pub fn copy_buffer(device: &ash::Device, command_pool: vk::CommandPool, submit_queue: vk::Queue, src_buffer: vk::Buffer, dst_buffer: vk::Buffer, size: vk::DeviceSize) {
	//Create a short lived command buffer
	let command_buffer_info = vk::CommandBufferAllocateInfo {
		s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
		p_next: ptr::null(),
		command_pool, //Command pool from which command buffer is allocated
		level: vk::CommandBufferLevel::PRIMARY, //Primary or secondary. Primary command buffers can execute secondary command buffers, kinda like executing a function
		command_buffer_count: 1, //Number of command buffers to allocate. If doing multiple frames in flight, must have one for each framebuffer (one for each swapchain image)
		..Default::default()
	};

	//Allocate the command buffer
	let command_buffers = unsafe { device.allocate_command_buffers(&command_buffer_info).expect("Failed to allocate command buffers") };

	//Need to set up a fence that will go off when the copy is done
	let fence_info = vk::FenceCreateInfo {
		s_type: vk::StructureType::FENCE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::FenceCreateFlags::empty(),
		..Default::default()
	};

	//Create the fence
	let copy_fence = unsafe { device.create_fence(&fence_info, None).expect("Failed to create fence") };

	//Start with the command buffer begin info
	let command_buffer_begin_info = vk::CommandBufferBeginInfo {
		s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
		p_next: ptr::null(),
		flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, //This command buffer will only be used once (for the copy)
		p_inheritance_info: ptr::null(),
		..Default::default()
	};

	//Begin recording to the command buffer
	unsafe { device.begin_command_buffer(command_buffers[0], &command_buffer_begin_info).expect("Failed to begin recording to command buffer") };

	//Define the regions to copy - want to copy the entire buffer
	let copy_infos = [vk::BufferCopy {
		src_offset: 0,
		dst_offset: 0,
		size
	}];

	//Copy the src buffer into the dst buffer
	unsafe { device.cmd_copy_buffer(command_buffers[0], src_buffer, dst_buffer, &copy_infos) };

	//End recording to the command buffer
	unsafe { device.end_command_buffer(command_buffers[0]).expect("Failed to record to command buffer") };

	//Execute the command buffer right away
	let submit_infos = [vk::SubmitInfo {
		s_type: vk::StructureType::SUBMIT_INFO,
		p_next: ptr::null(),
		wait_semaphore_count: 0,
		p_wait_semaphores: ptr::null(),
		p_wait_dst_stage_mask: ptr::null(),
		command_buffer_count: 1,
		p_command_buffers: &command_buffers[0],
		signal_semaphore_count: 0,
		p_signal_semaphores: ptr::null(),
		..Default::default()
	}];

	//Submits command buffer to queue, signals fence when complete
	unsafe {device.queue_submit(submit_queue, &submit_infos, copy_fence).expect("Failed to submit command buffer to queue") };
	//Wait for the fence right away
	unsafe { device.wait_for_fences(&[copy_fence], true, std::u64::MAX).expect("Failed to wait for fence") };

	//Clean up the command buffer and the fence
	unsafe { device.free_command_buffers(command_pool, &command_buffers) };
	unsafe { device.destroy_fence(copy_fence, None) };
}


//Finds the right type of GPU memory for whatever we want to do, return the index for that memory type
//Chooses based on memory types. Doesn't worry about the specific memory heap at the moment
fn find_memory_type_index(instance: &ash::Instance, physical_device: vk::PhysicalDevice, type_filter: u32, required_memory_properties: vk::MemoryPropertyFlags) -> u32 {
	//Query for available types of memory
	//This will give a "VkPhysicalDeviceMemoryProperties" with the available memory types and heaps
	let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };

	//Loop through the memory types, check against the type filter, also check that it's suitable
	for (memory_index, memory_type) in memory_properties.memory_types.iter().enumerate() {
		let memory_type_bits = 1 << memory_index; //Memory type bits contains a bit set for every supported memory type for the resource, corresponding to the memory index i
		
		//Check to make sure the memory type bits are the desired ones from the type filter, and also it has at least the required properties
		//Basically, checking that it satisfies the buffer requirements and the physical device requirements
		if (type_filter & memory_type_bits) > 0 && (memory_type.property_flags & required_memory_properties) == required_memory_properties {
			return memory_index as u32
		}
	}
	panic!("Failed to find suitable memory type");
}