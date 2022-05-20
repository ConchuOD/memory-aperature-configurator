// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use text_io::read;
mod soc;
use crate::soc::SoC;
use crate::soc::Aperture;

fn hex_to_mib(hex: u64) -> u64
{
	return hex / (2_u64.pow(10).pow(2))
}

fn display_segs(memory_apertures: &Vec<soc::MemoryAperture>)
{
	print!("For insertion into config.yaml:\nseg-reg-config: {{ ");
	for memory_aperture in memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       soc::hw_start_addr_to_seg(memory_aperture.hardware_addr, memory_aperture.bus_addr)
		);
	}
	print!("}}\n");
}

fn display_status(board: &mut soc::MPFS)
{
	let mut config_is_valid: bool = true;
	println!("Description | bus address | aperture hw start | aperture hw end | aperature size");
	for memory_aperture in &board.memory_apertures {
		let aperature_start = memory_aperture.get_hw_start_addr(board.total_system_memory);
		let aperature_end = memory_aperture.get_hw_end_addr(board.total_system_memory);

		if aperature_start.is_err() || aperature_end.is_err() {
			println!("| {}\t | {:#012x?} | invalid | invalid | n/a MiB",
				 memory_aperture.description,
				 memory_aperture.bus_addr
			);

			config_is_valid = false;
			continue
		}

		let aperature_size = aperature_end.as_ref().unwrap() - aperature_start.as_ref().unwrap();
		println!("| {}\t | {:#012x?} | {:#012x?} | {:#012x?} | {} MiB",
			 memory_aperture.description,
			 memory_aperture.bus_addr,
			 aperature_start.as_ref().unwrap(),
			 aperature_end.as_ref().unwrap(),
			 hex_to_mib(aperature_size)
		);
	}

	if config_is_valid {
		display_segs(&board.memory_apertures)
	}
}

#[derive(Copy, Clone)]
struct State {
	state_id: States,
	previous_state_id: States
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
enum States {
	Init,
	SelectAperature,
	WaitForInput,
	SelectOperation,
	Exit
}

fn init_handler(current_state: State, board: &mut soc::MPFS) -> State
{
	if current_state.previous_state_id == States::Exit {
		println!("Default Memory Aperatures:");
		display_status(board);
	}

	board.total_system_memory = 0x8000_0000;
	print!("\nEnter total system memory in hex:\n");

	return State {state_id: States::WaitForInput, previous_state_id: current_state.state_id}
}

fn select_aperature_handler(current_state: State, board: &mut soc::MPFS) -> State
{	
	if current_state.previous_state_id != States::Init {
		display_status(board);
	}

	let aperature_iter = board.memory_apertures.iter();
	print!("\n\nPress to edit: \n");
	for (i, memory_aperture) in aperature_iter.enumerate() {
		println!("{}: {}",
		       i,
		       memory_aperture.description
		);
	}

	return State {state_id: States::WaitForInput, previous_state_id: current_state.state_id}
}

fn wait_for_input_handler(current_state: State, board: &mut soc::MPFS) -> State
{	
	let mut next_state = State {state_id: States::Exit, previous_state_id: current_state.state_id};

	if current_state.previous_state_id == States::Init {
		let memory_raw: String = read!("{}");
		let memory_trimmed = memory_raw.trim_start_matches("0x");
		let memory = u64::from_str_radix(memory_trimmed, 16);
		if memory.is_err() {
			println!("Invalid amount of system memory. Please enter a hex number");
			next_state.state_id = States::Init;
			return next_state; 
		}

		board.total_system_memory = memory.unwrap();

		next_state.state_id = States::SelectAperature;
		return next_state;
	}

	if current_state.previous_state_id == States::SelectAperature {
		let aperature_id: u64 = read!();
		if aperature_id as usize >= board.memory_apertures.len() {
		//invalid aperature
			next_state.state_id = States::SelectAperature;
			return next_state;
		}
		
		board.current_aperture_id = Some(aperature_id as usize);

		next_state.state_id = States::SelectOperation;
		return next_state;
	}

	if current_state.previous_state_id == States::SelectOperation {
		let addr_raw: String = read!("{}");
		let addr_trimmed = addr_raw.trim_start_matches("0x");
		let addr = u64::from_str_radix(addr_trimmed, 16);
		if addr.is_err() {
			println!("Invalid address. Please enter a hex number");
			next_state.state_id = States::SelectOperation;
			return next_state; 
		}

		let current_aperture_id = board.current_aperture_id.unwrap();
		if board.set_hw_start_addr_by_id(addr.unwrap(), current_aperture_id).is_err() {
			println!("Failed setting hardware start address: requested address was greater than the total system memory.\nTry again - please enter a new hex number:");
			next_state.state_id = current_state.state_id;
			next_state.previous_state_id = States::SelectOperation;

			return next_state;
		}
	
		next_state.state_id = States::SelectAperature;
		return next_state;
	}

	return next_state
}

fn select_operation_handler(current_state: State, board: &mut soc::MPFS) -> State
{	
	display_status(board);
	print!("\nSet hardware start address: \n");

	return State {state_id: States::WaitForInput, previous_state_id: current_state.state_id}
}

fn exit_handler(current_state: State, board: &mut soc::MPFS) -> State
{
	print!("{{ ");
	for memory_aperture in &board.memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       soc::hw_start_addr_to_seg(memory_aperture.hardware_addr, memory_aperture.bus_addr)
		);
	}
	print!("}}\n");
	std::process::exit(0)
}

const STATE_HANDLERS: [fn(State, &mut soc::MPFS) -> State; 5] = [
	init_handler,
	select_aperature_handler,
	wait_for_input_handler,
	select_operation_handler,
	exit_handler
];

fn get_next_state(current_state: State, board: &mut soc::MPFS) -> State 
{
	let next_state = STATE_HANDLERS[current_state.state_id as usize](current_state, board);
	return next_state
}

fn main() {
	let mut next_state = State {
		state_id: States::Init,
		previous_state_id: States::Exit
	};

	let mut board = soc::MPFS::default();
	loop {
		next_state = get_next_state(next_state, &mut board);
	}
}
