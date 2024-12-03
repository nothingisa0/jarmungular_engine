use std::collections::HashSet;
use winit::keyboard::Key;

//Structure to hold the position of a midi knob
#[derive(Debug, Clone, Copy)]
pub struct Knob {
	current_position: f32,
}

impl Knob {
	pub fn init() -> Knob {
		Knob {
			current_position: -1.0,
		}
	}
}

//Bome midi translator keypress setup. Give the value in binary and do keypresses based on it
pub fn print_midi_config() {
	println!("[Project]\nVersion=1\n\n[Preset.0]\nName=Jarmungular Preset\nActive=1");

	for i_knob in 21..29 {
		//For every knob position
		for i in 0..128 {
			let mut pressed_keys_string = String::new();
			let mut released_keys_string = String::new();
			let mut key_commands = 2; //Will be at least 2 for the m press/release

			//Make a string for all the pressed/unpressed keys that corresponds to the binary midi value
			//0-8 on the keyboard will correspond to 0-8 as binary positions
			for n in 0..8 {
				if i >> n & 0b00000001 != 0 {
					let pressed_key = format!("03{n}");
					let released_key = format!("23{n}");

					pressed_keys_string.push_str(&pressed_key);
					released_keys_string.push_str(&released_key);

					key_commands += 2;
				}
			}
			
			//Do the same for the knob id. There's only 8 knobs, so it can be encoded in the 3 keys "9," "-," and "=" 
			let knob_id = i_knob - 21;
			//Encode ones place in "9"
			if knob_id & 0b00000001 != 0 {
				pressed_keys_string.push_str("039");
				released_keys_string.push_str("239");

				key_commands += 2;
			}
			//Encode twos place in "-"
			if knob_id & 0b00000010 != 0 {
				pressed_keys_string.push_str("0BD");
				released_keys_string.push_str("2BD");

				key_commands += 2;
			}
			//Encode fours place in "="
			if knob_id & 0b00000100 != 0 {
				pressed_keys_string.push_str("0BB");
				released_keys_string.push_str("2BB");

				key_commands += 2;
			}

			let command_id = (i_knob - 21) * 128 + i;

			//Print everything needed to setup the midi translation
			println!("Name{command_id}=Knob {knob_id}");
			println!("Incoming{command_id}=MID1B0{i_knob:02x}{i:02x} ");
			println!("Outgoing{command_id}=KAM10100KSQ100{key_commands:02x}{pressed_keys_string}04D24D{released_keys_string}"); //"04D24D23" is press + unpress "m"
			println!("Options{command_id}=Actv01Stop00OutO00");
		}
	}
	println!();
}

//Take in a hashset of keys held. These will correspond to midi keyboard stuff
//Returns the knob id so stuff can be printed out with the right name
pub fn midi_debug_controls(held_keys: &HashSet<Key>, knobs: &mut [Knob], values_to_adjust: &mut [&mut f32], bounds: Vec<(f32, f32)>) -> usize {
	let mut knob_id = 0b00000000;
	let mut midi_value = 0b00000000;

	//Get the knob id from the held down keys
	if held_keys.contains(&Key::Character("9".into())) {
		knob_id += 0b00000001;
	}
	if held_keys.contains(&Key::Character("-".into())) {
		knob_id += 0b00000010;
	}
	if held_keys.contains(&Key::Character("=".into())) {
		knob_id += 0b00000100;
	}

	//Get the value from the held down keys
	for n in 0..8 {
		if held_keys.contains(&Key::Character(n.to_string().into())) {
			midi_value += 0b00000001 << n;
		}
	}

	//Make sure nothing will break
	if knob_id >= values_to_adjust.len() || values_to_adjust.len() != bounds.len() {
		println!("{:?}", "Failed midi knob stuff");
		return 0 //This should be a custom error, but it's debug so whatever
	}

	let new_knob_position = midi_value as f32;

	//If the knob is still initialized, set its position now
	if knobs[knob_id].current_position < 0.0 {
		knobs[knob_id].current_position = new_knob_position;
	}

	let step = (bounds[knob_id].1 - bounds[knob_id].0) / 127.0;

	//Set the value that needed adjustment
	*values_to_adjust[knob_id] = bounds[knob_id].0 + new_knob_position * step;

	//Set the knob position
	knobs[knob_id].current_position = new_knob_position;

	//Return stuff for printing
	knob_id
}