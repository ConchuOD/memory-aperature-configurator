use std::error::Error;
use std::fmt;
#[derive(Debug)]
pub struct SegError {
}

impl fmt::Display for SegError {
fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	write!(f, "SegError is here!")
}
}

impl Error for SegError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
	None
}
}

pub trait Aperture {
	fn get_hw_start_addr(&self, total_system_memory: u64) -> Result<u64, SegError>;
	fn get_hw_end_addr(&self, total_system_memory: u64) -> Result<u64, SegError>;
	fn set_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64) -> Result<(), SegError>;
}

#[derive(Debug)]
pub struct MemoryApertureError;
pub struct MemoryAperture {
	pub description: String,
	pub bus_addr: u64,
	pub hardware_addr: u64,
	pub aperture_size: u64,
	pub reg_name: String
}

impl Aperture for MemoryAperture {

	fn get_hw_start_addr(&self, total_system_memory: u64) -> Result<u64, SegError> {
		if self.hardware_addr > total_system_memory{
			return Err(SegError {})
		}
		return Ok(self.hardware_addr)
	}

	fn get_hw_end_addr(&self, total_system_memory: u64) -> Result<u64, SegError> {
	// the last hardware addr decided by whichever is lower:
	// - the addr of the "highest" physical memory on the system
	// - the and of the aperture into memory on this part of the bus

		let aperture_max = self.hardware_addr + self.aperture_size;
		if aperture_max > total_system_memory {
			return Ok(total_system_memory)
		} else {
			return Ok(aperture_max)
		}
	}

	fn set_hw_start_addr(&mut self, total_system_memory: u64, new_start_addr: u64) -> Result<(), SegError> {
		if new_start_addr < total_system_memory {
			self.hardware_addr = new_start_addr;
			return Ok(())
		} else {
			return Err(SegError {})
		}
	}
}

pub trait SoC {
	fn get_hw_start_addr_by_id(&self, total_system_memory: u64, id: usize) -> Result<u64, SegError>;
	fn get_hw_end_addr_by_id(&self, total_system_memory: u64, id: usize) -> Result<u64, SegError>;
	fn set_hw_start_addr_by_id(&mut self, new_start_addr: u64, id: usize) -> Result<(), SegError>;
}

pub struct MPFS {
	pub total_system_memory: u64,
	pub memory_apertures: Vec<MemoryAperture>,
	pub current_aperture_id: Option<usize>
}

impl SoC for MPFS {

	fn get_hw_start_addr_by_id(&self, total_system_memory: u64, id: usize) -> Result<u64, SegError> {
		return self.memory_apertures[id].get_hw_start_addr(self.total_system_memory)
	}

	fn get_hw_end_addr_by_id(&self, total_system_memory: u64, id: usize) -> Result<u64, SegError> {
		return self.memory_apertures[id].get_hw_end_addr(self.total_system_memory)
	}

	fn set_hw_start_addr_by_id(&mut self, new_start_addr: u64, id: usize) -> Result<(), SegError> {
		return self.memory_apertures[id].set_hw_start_addr(self.total_system_memory, new_start_addr);
	}
}

impl Default for MPFS {
	fn default() -> MPFS {
		MPFS {
			total_system_memory: 0x8000_0000,
			current_aperture_id: None,
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

pub fn seg_to_hw_start_addr(seg: u64, bus_addr: u64) -> u64
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
	return bus_addr - temp
}

pub fn hw_start_addr_to_seg(hw_start_addr: u64, bus_addr: u64) -> u64
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
	return (0x4000 - temp) | 0x4000
}
