// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use device_tree;

#[derive(Clone, Debug)]
pub struct MemoryNode {
	address: u64,
	size: u64,
	label: String,
}

pub fn dt_get_memory_nodes(root_node: device_tree::Node)
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
