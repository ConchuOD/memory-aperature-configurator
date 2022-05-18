#![allow(unused_variables)]

trait Aperture {
	fn get_hw_start_addr(&self) -> u64;
	fn get_hw_end_addr(&self, total_system_memory: u64) -> u64;
	fn update_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64);
}

struct MemoryAperture {
	description: String,
	bus_addr: u64,
	hardware_addr: u64,
	aperture_size: u64,
	seg_reg: u64,
	reg_name: String
}

impl Aperture for MemoryAperture {

	fn get_hw_start_addr(&self) -> u64 {
	// the last hardware addr decided by whichever is lower:
	// - the addr of the "highest" physical memory on the system
	// - the and of the aperture into memory on this part of the bus

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

	fn update_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64) {
		if new_start_addr < total_system_memory {
			self.hardware_addr = new_start_addr;
		}
		// else
		// 	return an error
	}
}

struct MPFS {
	total_system_memory: u64,
	memory_apertures: Vec<MemoryAperture>
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
					hardware_addr: 0x0,
					aperture_size: 0x40_000_0000,
					seg_reg: 0x0
				},
				MemoryAperture {
					description: "64-bit non-cached".to_string(),
					reg_name: "seg1_3".to_string(),
					bus_addr: 0x14_0000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
					seg_reg: 0x0
				},
				MemoryAperture {
					description: "64-bit WCB\t".to_string(),
					reg_name: "seg1_5".to_string(),
					bus_addr: 0x18_0000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
					seg_reg: 0x0
				},
				MemoryAperture {
					description: "32-bit cached\t".to_string(),
					reg_name: "seg0_0".to_string(),
					bus_addr: 0x8000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x4000_0000,
					seg_reg: 0x0
				},
				MemoryAperture {
					description: "32-bit non-cached".to_string(),
					reg_name: "seg1_2".to_string(),
					bus_addr: 0xC000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x1000_0000,
					seg_reg: 0x0
				},
				MemoryAperture {
					description: "32-bit WCB\t".to_string(),
					reg_name: "seg1_4".to_string(),
					bus_addr: 0xD000_0000,
					hardware_addr: 0x0,
					aperture_size: 0x1000_0000,
					seg_reg: 0x0
				}
			]
		}
	}
}

fn hex_to_mib(hex: u64) -> u64
{
	hex / (2_u64.pow(10).pow(2))
}

fn format_segs(memory_apertures: Vec<MemoryAperture>)
{
	print!("{{ ");
	for memory_aperture in memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       memory_aperture.seg_reg
		);
	}
	print!("}}\n");
}

fn seg_to_hw_start_addr(seg: u64) -> u64
{
	1_u64
}

fn main() {
	let mut board = MPFS::default();
	board.total_system_memory = 0x8000_0000;
	println!("Default Memory Aperatures\n");
	println!("Description | bus address | aperture hw start | aperture hw end | aperature size\n");
	for memory_aperture in &board.memory_apertures {
		println!("| {}\t | {:#012x?} | {:#012x?} | {:#012x?} | {} MiB",
			 memory_aperture.description,
			 memory_aperture.bus_addr,
			 memory_aperture.get_hw_start_addr(),
			 memory_aperture.get_hw_end_addr(board.total_system_memory),
			 hex_to_mib(memory_aperture.get_hw_end_addr(board.total_system_memory) - memory_aperture.get_hw_start_addr())
		);
	}
	format_segs(board.memory_apertures);
}
