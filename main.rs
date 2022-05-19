// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
use text_io::read;

trait Aperture {
	fn get_hw_start_addr(&self) -> u64;
	fn get_hw_end_addr(&self, total_system_memory: u64) -> u64;
	fn set_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64);
}

struct MemoryAperture {
	description: String,
	bus_addr: u64,
	hardware_addr: u64,
	aperture_size: u64,
	reg_name: String
}

impl Aperture for MemoryAperture {

	fn get_hw_start_addr(&self) -> u64 {
		self.hardware_addr
	}

	fn get_hw_end_addr(&self, total_system_memory: u64) -> u64 {
	// the last hardware addr decided by whichever is lower:
	// - the addr of the "highest" physical memory on the system
	// - the and of the aperture into memory on this part of the bus

		let aperture_max = self.hardware_addr + self.aperture_size;
		if aperture_max > total_system_memory {
			total_system_memory
		} else {
			aperture_max
		}
	}

	fn set_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64) {
		if new_start_addr < total_system_memory {
			self.hardware_addr = new_start_addr;
		}
		// else
		// 	return an error
	}
}

trait SoC {
	fn get_hw_start_addr_by_id(&self, id: usize) -> u64;
	fn get_hw_end_addr_by_id(&self, id: usize) -> u64;
	fn set_hw_start_addr_by_id(&mut self, new_start_addr: u64, id: usize);
}

struct MPFS {
	total_system_memory: u64,
	memory_apertures: Vec<MemoryAperture>,
	current_aperture_id: Option<usize>
}

impl SoC for MPFS {

	fn get_hw_start_addr_by_id(&self, id: usize) -> u64 {
		self.memory_apertures[id].get_hw_start_addr()
	}

	fn get_hw_end_addr_by_id(&self, id: usize) -> u64 {
		self.memory_apertures[id].get_hw_end_addr(self.total_system_memory)
	}

	fn set_hw_start_addr_by_id(&mut self, new_start_addr: u64, id: usize) {
		self.memory_apertures[id].set_hw_start_addr(self.total_system_memory, new_start_addr)
	}
}

impl Default for MPFS {
	fn default() -> MPFS {
		MPFS {
			total_system_memory: 0x8000_0000,
			memory_apertures: vec![
				MemoryAperture {
					description: "64-bit cached\t".to_string(),
					reg_name: "seg0_1".to_string(),
					bus_addr: 0x10_0000_0000,
					hardware_addr: 0x00_0200_0000,
					aperture_size: 0x40_000_0000,
				},
				MemoryAperture {
					description: "64-bit non-cached".to_string(),
					reg_name: "seg1_3".to_string(),
					bus_addr: 0x14_0000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
				},
				MemoryAperture {
					description: "64-bit WCB\t".to_string(),
					reg_name: "seg1_5".to_string(),
					bus_addr: 0x18_0000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
				},
				MemoryAperture {
					description: "32-bit cached\t".to_string(),
					reg_name: "seg0_0".to_string(),
					bus_addr: 0x8000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
				},
				MemoryAperture {
					description: "32-bit non-cached".to_string(),
					reg_name: "seg1_2".to_string(),
					bus_addr: 0xC000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x1000_0000,
				},
				MemoryAperture {
					description: "32-bit WCB\t".to_string(),
					reg_name: "seg1_4".to_string(),
					bus_addr: 0xD000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x1000_0000,
				}
			],
			current_aperture_id: None
		}
	}
}

fn seg_to_hw_start_addr(seg: u64, bus_addr: u64) -> u64
{
	let mut temp = seg;

	if (temp & 0x4000) == 0 {
	// if that bit isnt set, either this seg register is:
	// - 0x0 (in which case the hw addr == the bus addr)
	// - invalid (so treat as zero to match the bootloader's behaviour)
		return bus_addr
	}

	temp &= 0x3FFF;
	temp = 0x4000 - temp;
	temp <<= 24;
	bus_addr - temp
}

fn hw_start_addr_to_seg(hw_start_addr: u64, bus_addr: u64) -> u64
{
	if bus_addr == hw_start_addr {
	// a seg register is effectively how much we need to subtract from the
	// bus addr to get the hw addr (not /quite/, but sorta) so if they're
	// the same, then the seg register is 0x0
		return 0x0
	}

	let mut temp = bus_addr;
	temp -= hw_start_addr;
	temp >>= 24;
	(0x4000 - temp) | 0x4000
}

fn hex_to_mib(hex: u64) -> u64
{
	hex / (2_u64.pow(10).pow(2))
}

fn display_segs(memory_apertures: &Vec<MemoryAperture>)
{
	print!("For insertion into config.yaml:\nseg-reg-config: {{ ");
	for memory_aperture in memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       hw_start_addr_to_seg(memory_aperture.hardware_addr, memory_aperture.bus_addr)
		);
	}
	print!("}}\n");
}

fn display_status(board: &mut MPFS)
{
	println!("Description | bus address | aperture hw start | aperture hw end | aperature size");
	for memory_aperture in &board.memory_apertures {
		println!("| {}\t | {:#012x?} | {:#012x?} | {:#012x?} | {} MiB",
			 memory_aperture.description,
			 memory_aperture.bus_addr,
			 memory_aperture.get_hw_start_addr(),
			 memory_aperture.get_hw_end_addr(board.total_system_memory),
			 hex_to_mib(memory_aperture.get_hw_end_addr(board.total_system_memory)
			 	    - memory_aperture.get_hw_start_addr())
		);
	}
	display_segs(&board.memory_apertures)
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

fn init_handler(current_state: State, board: &mut MPFS) -> State
{
	board.total_system_memory = 0x8000_0000;
	println!("Default Memory Aperatures:");
	display_status(board);
	print!("\n\nEnter total system memory in hex:\n");
	return State {state_id: States::WaitForInput, previous_state_id: current_state.state_id}
}

fn select_aperature_handler(current_state: State, board: &mut MPFS) -> State
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

fn wait_for_input_handler(current_state: State, board: &mut MPFS) -> State
{	
	if current_state.previous_state_id == States::Init {
		let total_system_memory_raw: String = read!("{}");
		let total_system_memory = total_system_memory_raw.trim_start_matches("0x");
		board.total_system_memory = u64::from_str_radix(total_system_memory, 16).unwrap();

		return State {state_id: States::SelectAperature, previous_state_id: current_state.state_id}
	}

	if current_state.previous_state_id == States::SelectAperature {
		let aperature_id: u64 = read!();
		if aperature_id as usize >= board.memory_apertures.len() {
		//invalid aperature
			return State {state_id: States::SelectAperature, previous_state_id: current_state.state_id}
		}
		
		board.current_aperture_id = Some(aperature_id as usize);
		return State {state_id: States::SelectOperation, previous_state_id: current_state.state_id}
	}

	if current_state.previous_state_id == States::SelectOperation {
		let hw_start_addr_raw: String = read!("{}");
		let hw_start_addr = hw_start_addr_raw.trim_start_matches("0x");
		board.set_hw_start_addr_by_id(u64::from_str_radix(hw_start_addr, 16).unwrap(), board.current_aperture_id.unwrap());
	
		return State {state_id: States::SelectAperature, previous_state_id: current_state.state_id}
	}

	return State {state_id: States::Exit, previous_state_id: current_state.state_id}
}

fn select_operation_handler(current_state: State, board: &mut MPFS) -> State
{	
	display_status(board);
	print!("\nSet hardware start address: \n");

	return State {state_id: States::WaitForInput, previous_state_id: current_state.state_id}
}

fn exit_handler(current_state: State, board: &mut MPFS) -> State
{
	print!("{{ ");
	for memory_aperture in &board.memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       hw_start_addr_to_seg(memory_aperture.hardware_addr, memory_aperture.bus_addr)
		);
	}
	print!("}}\n");
	std::process::exit(0)
}

const STATE_HANDLERS: [fn(State, &mut MPFS) -> State; 5] = [
	init_handler,
	select_aperature_handler,
	wait_for_input_handler,
	select_operation_handler,
	exit_handler
];

fn get_next_state(current_state: State, board: &mut MPFS) -> State 
{
	let next_state = STATE_HANDLERS[current_state.state_id as usize](current_state, board);
	return next_state
}

fn main() {
	let mut next_state = State {
		state_id: States::Init,
		previous_state_id: States::Exit
	};

	let mut board = MPFS::default();
	loop {
		next_state = get_next_state(next_state, &mut board);
	}
}
