// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use device_tree;
use std::io::Read;
use std::fs;

use crate::soc::Aperture;
use crate::soc::MemoryAperture;
use crate::soc::MPFS;
use crate::soc::SegError;

#[derive(Clone, Debug)]
pub struct MemoryNode {
	pub address: u64,
	pub size: u64,
	pub label: String,
}

pub trait NoGoodNameYet {
	fn to_strings(&self, board: &mut MPFS) -> Vec<String>;

	fn get_hw_start_addr
	(&self, apertures: &mut Vec<MemoryAperture>) -> Result<u64, SegError>;
}

impl NoGoodNameYet for MemoryNode {
	fn to_strings(&self, board: &mut MPFS) -> Vec<String>
	{
		let mut strings = Vec::new();
		let hw_address = self.get_hw_start_addr(&mut board.memory_apertures);

		strings.push(self.label.clone());
		strings.push(format!("{:#012x}", self.address).to_string());
		strings.push(format!("{:#012x}", self.size).to_string());
		
		if hw_address.is_err() {
			strings.push(format!("{:#012x}", 0).to_string());
			strings.push(format!("{:#012x}", 0).to_string());
		} else {
			let hw_address = hw_address.unwrap();
			strings.push(format!("{:#012x}", hw_address).to_string());
			strings.push(format!("{:#012x}", hw_address + self.size - 1).to_string());
		}

		return strings.clone()
	}

	fn get_hw_start_addr
	(&self, apertures: &mut Vec<MemoryAperture>) -> Result<u64, SegError>
	{
		for aperture in apertures.iter_mut() {
			let hw_start_addr = aperture.get_region_hw_start_addr(self.address,
									      self.size);
			if hw_start_addr.is_none() {
				continue
			}

			return Ok(hw_start_addr.unwrap())
		}

		dbg!("no overlapping region found for {:?} {:?}", apertures, self);

		return Err(SegError {})
	}

}

pub fn memory_nodes_to_strings(board: &mut MPFS, nodes: Vec<MemoryNode>) -> Vec<Vec<String>>
{
	//I'm sure this should be a closure or w/e
	let mut strings = Vec::new();
	for node in nodes {
		strings.push(node.to_strings(board));
	}
	return strings.clone()
}

fn get_memory_nodes(root_node: device_tree::Node)
-> Result<Vec<MemoryNode>, Box<dyn std::error::Error>>
{
	//TODO: parse size/address cells
	//TODO: consider disabled nodes
	let size_cells = 2;
	let address_cells = 2;
	let mut memory_nodes: Vec<MemoryNode> = Vec::new();
	let children = root_node.children.iter();
	for child in children {
		let device_type = child.prop_str("device_type");
		if device_type.is_err() {
			continue;
		}
		if device_type.unwrap() == "memory" {
			let reg = child.prop_raw("reg");
			if reg.is_none() {
				continue;
			}
			let (addr_vec, size_vec) = reg.unwrap().split_at(8);
			let addr = u64::from_be_bytes(addr_vec.try_into().unwrap());
			let size = u64::from_be_bytes(size_vec.try_into().unwrap());
			let node = MemoryNode {
				label: child.name.clone(),
				address: addr,
				size: size,
			};
			memory_nodes.push(node);
		}
	}
	println!("{:?}", memory_nodes);
	return Ok(memory_nodes.clone())
}

pub fn dtb_get_memory_nodes(dtb_file: String)
-> Result<Option<Vec<MemoryNode>>, Box<dyn std::error::Error>>
{
	let mut dtb_handle = fs::File::open(dtb_file)?;
	let mut dtb = Vec::new();
	dtb_handle.read_to_end(&mut dtb)?;
	let dt = device_tree::DeviceTree::load(dtb.as_slice())
			.or(Err("bad dtb"))?;
	let root_node = dt.root;
	return Ok(Some(get_memory_nodes(root_node)?));
}

