use crate::soc;
use crate::soc::SoC;

#[derive(Clone)]
pub struct State {
	state_id: States,
	previous_state_id: States,
	pub command_text: String
}
impl Default for State {
	fn default() -> State {
		State {
			state_id: States::Init,
			previous_state_id: States::Exit,
			command_text: "Press Enter to begin...".to_string()
		}
	}
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Debug)]
pub enum States {
	Init,
	SelectAperature,
	WaitForInput,
	SelectOperation,
	Exit
}

fn init_handler
(current_state: State, board: &mut soc::MPFS, input: Option<String>) -> State
{
	board.total_system_memory = 0x8000_0000;

	return State {
		state_id: States::WaitForInput,
		previous_state_id: current_state.state_id,
		command_text: format!("Enter total system memory in hex:")
	}
}

fn select_aperature_handler
(current_state: State, board: &mut soc::MPFS, input: Option<String>) -> State
{	
	return State {
		state_id: States::WaitForInput,
		previous_state_id: current_state.state_id,
		command_text: "Enter an aperature ID to edit:".to_string()
	}
}

fn wait_for_input_handler
(current_state: State, board: &mut soc::MPFS, input: Option<String>) -> State
{	
	let mut next_state = State {
		state_id: States::WaitForInput,
		previous_state_id: current_state.previous_state_id,
		command_text: current_state.command_text
	};

	if input.is_none() {
		return next_state;
	}

	if current_state.previous_state_id == States::Init {
		let memory_raw: String = input.unwrap();
		let memory_trimmed = memory_raw.trim_start_matches("0x");
		let memory = u64::from_str_radix(memory_trimmed, 16);
		if memory.is_err() {
			next_state.command_text = format!(
					"Invalid amount of system memory ({}). \
					Please enter a hex number",
					memory_raw
				)
				.to_string();
			next_state.state_id = States::WaitForInput;
			return next_state;
		}

		board.total_system_memory = memory.unwrap();

		next_state.state_id = States::SelectAperature;
		return next_state;
	}

	if current_state.previous_state_id == States::SelectAperature {
		let aperature_id_raw: String = input.unwrap();
		let aperature_id_trimmed = aperature_id_raw.trim_start_matches("0x");
		let aperature_id = u64::from_str_radix(aperature_id_trimmed, 16);
		if aperature_id.is_err() {
			println!("Invalid address. Please enter a hex number");
			next_state.state_id = States::SelectOperation;
			return next_state;
		}
		let id = aperature_id.unwrap();
		if id as usize >= board.memory_apertures.len() {
			next_state.state_id = States::SelectAperature;
			next_state.command_text = format!("Invalid aperature ID").to_string();
			return next_state;
		}
		
		board.current_aperture_id = Some(id as usize);

		next_state.state_id = States::SelectOperation;
		return next_state;
	}

	if current_state.previous_state_id == States::SelectOperation {
		let addr_raw: String = input.unwrap();
		let addr_trimmed = addr_raw.trim_start_matches("0x");
		let addr = u64::from_str_radix(addr_trimmed, 16);
		if addr.is_err() {
			println!("Invalid address. Please enter a hex number");
			next_state.state_id = States::SelectOperation;
			return next_state;
		}

		let current_aperture_id = board.current_aperture_id.unwrap();
		if board.set_hw_start_addr_by_id(addr.unwrap(), current_aperture_id).is_err() {
			println!(
				"Failed setting hardware start address: requested address was \
				greater than the total system memory.\n\
				Try again - please enter a new hex number:"
			);
			next_state.state_id = current_state.state_id;
			next_state.previous_state_id = States::SelectOperation;

			return next_state;
		}
	
		next_state.state_id = States::SelectAperature;
		return next_state;
	}

	return next_state
}

fn select_operation_handler
(current_state: State, board: &mut soc::MPFS, input: Option<String>) -> State
{	
	let current_aperture_id = board.current_aperture_id.unwrap();

	let next_state = State {
		state_id: States::WaitForInput,
		previous_state_id: current_state.state_id,
		command_text: format!(
			"Set hardware start address for {}:", 
			board.memory_apertures[current_aperture_id].description
		)
	};

	return next_state
}

fn exit_handler(current_state: State, board: &mut soc::MPFS, input: Option<String>) -> State
{
	std::process::exit(0)
}

const STATE_HANDLERS: [fn(State, &mut soc::MPFS, input: Option<String>) -> State; 5] = [
	init_handler,
	select_aperature_handler,
	wait_for_input_handler,
	select_operation_handler,
	exit_handler
];

pub fn get_next_state(current_state: State, board: &mut soc::MPFS, input: &mut Vec<String>) -> State 
{
	let state_id = current_state.state_id as usize;
	let next_state = STATE_HANDLERS[state_id](current_state, board, input.pop());
	input.clear();

	return next_state
}