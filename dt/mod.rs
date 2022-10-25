// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use device_tree;

#[derive(Clone, Debug)]
pub struct MemoryNode {
	pub address: u64,
	pub size: u64,
	pub label: String,
}

fn memory_node_to_string(node: MemoryNode) -> Vec<String>
{
	let mut strings = Vec::new();
	strings.push(node.label);
	strings.push(format!("{:#12x}", node.address).to_string());
	strings.push(format!("{:#12x}", node.size).to_string());
	return strings.clone()
}

pub fn memory_nodes_to_strings(nodes: Vec<MemoryNode>) -> Vec<Vec<String>>
{
	//I'm sure this should be a closure or w/e
	let mut strings = Vec::new();
	for node in nodes {
		strings.push(memory_node_to_string(node));
	}
	return strings.clone()
}

pub fn get_memory_nodes(root_node: device_tree::Node)
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
