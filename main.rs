// SPDX-License-Identifier: MIT or GPL-2.0

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
			]
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

fn format_segs(memory_apertures: Vec<MemoryAperture>)
{
	print!("{{ ");
	for memory_aperture in memory_apertures {
		print!("{}: {:#x?}, ",
		       memory_aperture.reg_name,
		       hw_start_addr_to_seg(memory_aperture.hardware_addr, memory_aperture.bus_addr)
		);
	}
	print!("}}\n");
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
			 hex_to_mib(memory_aperture.get_hw_end_addr(board.total_system_memory)
			 	    - memory_aperture.get_hw_start_addr())
		);
	}
	format_segs(board.memory_apertures);
	println!("{:#012x?}\n", seg_to_hw_start_addr(0x7002, 0x10_0000_0000));
	println!("{:#012x?}\n", hw_start_addr_to_seg(seg_to_hw_start_addr(0x7002, 0x10_0000_0000), 0x10_0000_0000));
}
