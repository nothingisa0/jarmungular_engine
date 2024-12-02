use glam::f32::{vec3, Vec3};

const MOVE_ACCEL: f32 = 0.1; //Acceleration when starting movement - units per timestep
const KICKOFF_BOOST_FACTOR: f32 = 2.0; //Factor to multiply move acceleration by when standing still

const MAX_MOVE_VELOCITY: f32 = 1.0; //Speed cap

const FRICTION_DECCEL: f32 = 0.1; //Decceleration from friction

//Structure to hold all the player info
pub struct Player {
	pos: Vec3, //Position
	vel: Vec3, //Velocity (position units per second)
}

impl Player {
	//Initialize a player with a position, zero velocity, and an attached camera
	pub fn new(pos: Vec3) -> Player {
		Player {
			pos,
			vel: vec3(0.0, 0.0, 0.0),
		}
	}

	//Gets player position
	pub fn get_pos(&self) -> Vec3 {
		self.pos
	}

	//Adds a given speed in a given direction
	pub fn move_grounded(&mut self, dir: Vec3) {
		//Normalize the direction
		let normalized_dir = dir.normalize_or_zero();

		//If already moving, just add the move acceleration
		//If stationary instead, give a bigger boost to kinda "kick off"
		if self.vel.length_squared() != 0.0 {
			self.vel += normalized_dir * MOVE_ACCEL;
			println!("Nonzero: {:?}", self.vel.length());
		} else {
			self.vel += normalized_dir * MOVE_ACCEL * KICKOFF_BOOST_FACTOR;
			println!("Zero, kickoff time!: {:?}", self.vel.length());
		}

		//Don't go higher than the speed cap
		let vel_mag = self.vel.length_squared();
		if vel_mag > MAX_MOVE_VELOCITY * MAX_MOVE_VELOCITY {
			self.vel = self.vel.normalize_or_zero() * MAX_MOVE_VELOCITY
		};
	}

	//Per timestep physics update for the player - change position based on velocity
	pub fn update(&mut self) {
		//Will need to add acceleration due to gravity here

		//Decceleration from friction
		self.friction_deccel();

		//Update positions
		self.pos += self.vel;

		//Since there's no collision yet, make sure z pos is >= 0
		if self.pos.y < 0.0 {
			self.pos.y = 0.0;
			self.vel.y = 0.0;
		};
	}

	//Decceleration from friction
	//Will need to make this tied to state eventually (less air friction than ground friction)
	fn friction_deccel(&mut self) {
		let vel_mag = self.vel.length_squared();
		if vel_mag > FRICTION_DECCEL * FRICTION_DECCEL {
			self.vel -= self.vel.normalize_or_zero() * FRICTION_DECCEL;
		} else {
			self.vel = Vec3::ZERO;
		};
	}
}
