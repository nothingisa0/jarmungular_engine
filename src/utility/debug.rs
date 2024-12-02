use std::collections::HashSet;
use winit::keyboard::Key;

//Bome midi translator keypress setup. Give the value in binary and do keypresses based on it
pub fn print_midi_config() {
	println!("[Project]\nVersion=1\n\n[Preset.0]\nName=Jarmungular Preset\nActive=1");

	//For every knob position
	for i in 0..128 as u8 {
		let mut pressed_keys_string = String::new();
		let mut released_keys_string = String::new();
		let mut key_commands = 2; //Will be at least 2 for the m press/release

		//Will make a string for all the pressed/unpressed keys
		for n in 0..8 {
			if i >> n & 0b00000001 != 0 {
				let pressed_key = String::from(format!("03{n}"));
				let released_key = String::from(format!("23{n}"));

				pressed_keys_string.push_str(&pressed_key);
				released_keys_string.push_str(&released_key);

				key_commands += 2;
			}
		}


		println!("Name{i}=Knob 1");
		println!("Incoming{i}=MID1B015{i:02x} ");
		println!("Outgoing{i}=KAM10100KSQ100{key_commands:02x}{pressed_keys_string}04D24D{released_keys_string}"); //"04D24D23" is press + unpress "m"
		println!("Options{i}=Actv01Stop00OutO00");
	}
	println!();
}

//Take in a hashset of keys held. These will correspond to midi keyboard stuff
pub fn midi_debug_controls(held_keys: &HashSet<Key>) {
	let mut midi_value = 0b00000000;

	//Get the value from the held down keys
	for n in 0..8 {
		if held_keys.contains(&Key::Character(n.to_string().into())) {
			midi_value += 0b00000001 << n;
		}
	}
}