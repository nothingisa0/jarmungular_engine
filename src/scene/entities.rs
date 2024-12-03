use glam::f32::{vec3, Vec3};

const MOVE_ACCEL: f32 = 0.1;
const KICKOFF_BOOST_FACTOR: f32 = 1.5;

const MAX_MOVE_VELOCITY: f32 = 1.0;

const FRICTION_DECCEL: f32 = 0.018;

//Make a structure to hold all the movement constants - will allow some freedom for changing them in debug
pub struct MoveConstants {
	pub move_accel: f32, //Acceleration when starting movement - units per timestep
	pub kickoff_boost_factor: f32, //Factor to multiply move acceleration by when standing still

	pub max_move_velocity: f32, //Speed cap

	pub friction_deccel: f32,  //Decceleration from friction
}

impl MoveConstants {
	fn init() -> MoveConstants {
		MoveConstants {
			move_accel: MOVE_ACCEL,
			kickoff_boost_factor: KICKOFF_BOOST_FACTOR,

			max_move_velocity: MAX_MOVE_VELOCITY,

			friction_deccel: FRICTION_DECCEL,
		}
	}
}


//Structure to hold all the player info
pub struct Player {
	pos: Vec3, //Position
	vel: Vec3, //Velocity (position units per second)

	pub move_constants: MoveConstants, //All the constants relevant to player movement
}

impl Player {
	//Initialize a player with a position, zero velocity, and an attached camera
	pub fn new(pos: Vec3) -> Player {
		let move_constants = MoveConstants::init();

		Player {
			pos,
			vel: vec3(0.0, 0.0, 0.0),

			move_constants,
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
			self.vel += normalized_dir * self.move_constants.move_accel;
		} else {
			self.vel += normalized_dir * self.move_constants.move_accel * self.move_constants.kickoff_boost_factor;
		}

		//Don't go higher than the speed cap
		let vel_mag = self.vel.length_squared();
		if vel_mag > self.move_constants.max_move_velocity * self.move_constants.max_move_velocity {
			self.vel = self.vel.normalize_or_zero() * self.move_constants.max_move_velocity
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
		if vel_mag > self.move_constants.friction_deccel * self.move_constants.friction_deccel {
			self.vel -= self.vel.normalize_or_zero() * self.move_constants.friction_deccel;
		} else {
			self.vel = Vec3::ZERO;
		};
	}
}
