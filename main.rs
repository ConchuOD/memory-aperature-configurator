// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use clap::Parser;
use crossterm::{
	event::{self, Event, KeyCode},
	terminal::{disable_raw_mode, enable_raw_mode},
};
use serde_yaml::Value;
use std::io;
use std::io::Read;
use std::time::Duration;
use std::fs;
use tui::{
	backend::{CrosstermBackend},
	Frame,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::Span, Terminal,
	widgets::{Block, Borders, Paragraph, Cell, Row, Table},
	widgets::canvas::{Canvas, Rectangle},
};

mod dt;
use crate::dt::MemoryNode;
use crate::dt::NoGoodNameYet;
mod soc;
use crate::soc::Aperture;
mod states;

fn hex_to_mib(hex: u64) -> u64
{
	return hex / (2_u64.pow(10).pow(2))
}

const READABLE_COLOURS: [Color; 6] =
[
	Color::LightRed,
	Color::LightGreen,
	Color::LightMagenta,
	Color::LightYellow,
	Color::LightCyan,
	Color::LightBlue
];

fn render_dt_node_table<B: tui::backend::Backend>
(board: &mut soc::MPFS, nodes: Option<Vec<MemoryNode>>, frame:&mut Frame<B>, display_rect: Rect)
{
	let selected_style = Style::default().add_modifier(Modifier::REVERSED);
	let header_cells = ["node name", "address", "size", "hw start", "hw end",]
		.iter()
		.map(|h|
			return
			Cell::from(*h)
			.style(Style::default())
		);

	let header = Row::new(header_cells).height(1).bottom_margin(1);

	if nodes.is_none() {
		return
	}

	let data = dt::memory_nodes_to_strings(board, nodes.unwrap());

	let rows = data.iter().map(|item| {
		let cells = item.iter().map(|c|
			return Cell::from(c.clone())
		);
		return Row::new(cells).height(1).bottom_margin(1)
	});

	let table =
		Table::new(rows)
		.header(header)
		.block(
			Block::default()
			.borders(Borders::ALL)

		)
		.style(Style::default())
		.highlight_style(selected_style)
		.highlight_symbol(">> ")
		.widths(&[
			Constraint::Percentage(20),
			Constraint::Percentage(20),
			Constraint::Percentage(20),
			Constraint::Percentage(20),
			Constraint::Percentage(20),
		]);

	frame.render_widget(table, display_rect);
}

fn render_seg_table<B: tui::backend::Backend>
(data: Vec<Vec<String>>, frame:&mut Frame<B>, display_rect: Rect)
{
	let selected_style = Style::default().add_modifier(Modifier::REVERSED);
	let header_cells =
		[
			"ID", "Register Name", "Description", "Bus Address",
			"Register Value", "Aperture HW Start", "Aperture HW End",
			"Aperature Size",
		 ]
		.iter()
		.map(|h|
			return
			Cell::from(*h)
			.style(Style::default())
		);

	let header = Row::new(header_cells).height(1).bottom_margin(1);
	let rows = data.iter().map(|item| {
		let cells = item.iter().map(|c|
			return Cell::from(c.clone())
		);
		return Row::new(cells).height(1).bottom_margin(1)
	});

	let table =
		Table::new(rows)
		.header(header)
		.block(
			Block::default()
			.borders(Borders::ALL)

		)
		.style(Style::default())
		.highlight_style(selected_style)
		.highlight_symbol(">> ")
		.widths(&[
			Constraint::Percentage(5),
			Constraint::Percentage(10),
			Constraint::Percentage(15),
			Constraint::Percentage(12),
			Constraint::Percentage(8),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
		]);

	frame.render_widget(table, display_rect);
}

#[derive(Clone)]
struct ApertureVis {
	rectangle: Option<Rectangle>,
	label: Option<char>,
	label_x: f64,
	label_y: f64
}

impl Default for ApertureVis {
	fn default() -> ApertureVis {
		return ApertureVis {
			rectangle: None,
			label: None,
			label_x: 0.0,
			label_y: 0.0
		}
	}
}

fn render_visualisation<B: tui::backend::Backend>
(board: &mut soc::MPFS, nodes: Option<Vec<MemoryNode>>, frame:&mut Frame<B>, display_rect: Rect)
{
	let border: f64 = 0.5;
	let mem_map_height: f64 = (display_rect.height) as f64 - 2.0 * border;
	let mem_map_width = 0.67 * (display_rect.width) as f64 - 2.0 * border;
	let mem_map_x = 1.0;
	let mem_map_y = 0.5;
	let px_per_byte: f64 = mem_map_height / board.total_system_memory as f64;

	let mut aperature_colours = READABLE_COLOURS.iter();

	let memory_map = Rectangle {
		x: mem_map_x,
		y: mem_map_y,
		width: mem_map_width,
		height: mem_map_height,
		color: Color::White,
	};

	let memory_apertures = board.memory_apertures.iter();
	let mut apertures: Vec<ApertureVis> = Vec::new();
	let num_apertures = 6.0; // this is a fixed property of the SoC
	let num_apertures = 7.0; // inc. by one for the dt node rendering
	let aperature_width = mem_map_width / (num_apertures + 1.0);
	let mut display_offset = aperature_width / num_apertures;

	for aperature in memory_apertures {
		let aperature_start = aperature.get_hw_start_addr(board.total_system_memory);
		let aperature_end = aperature.get_hw_end_addr(board.total_system_memory);
		let colour = *aperature_colours.next().unwrap(); // yeah, yeah this could crash
		let mut aperture_vis: ApertureVis = ApertureVis {
			label: aperature.reg_name.chars().last(),
			..Default::default()
		};

		let rectangle_x = mem_map_x + display_offset;

		aperture_vis.label_x = rectangle_x + 0.5 * aperature_width;
		aperture_vis.label_y = mem_map_y - 0.5;

		if aperature_start.is_ok() && aperature_end.is_ok() {
			let aperture_y: f64 = px_per_byte * aperature_start.unwrap() as f64;
			let aperture_height: f64 = px_per_byte * aperature_end.unwrap() as f64
						   - aperture_y;
			let rectangle = Rectangle {
				x: rectangle_x,
				y: mem_map_y + aperture_y,
				width: aperature_width,
				height: aperture_height,
				color: colour,
			};
			aperture_vis.rectangle = Some(rectangle);
		}
		apertures.push(aperture_vis.clone());
		display_offset += aperature_width + aperature_width / num_apertures;
	}

	if nodes.is_some() {
		let mut node_colours = READABLE_COLOURS.iter();
		for node in nodes.unwrap().iter() {
			let start_addr = node.get_hw_start_addr(&mut board.memory_apertures.clone());
			if start_addr.is_err() {
				break;
			}

			let colour = *node_colours.next().unwrap(); // yeah, yeah this could crash


			let start_addr = start_addr.unwrap();

			let mut node_vis = ApertureVis {
				label: "c".chars().last(),
				..Default::default()
			};

			let rectangle_x = mem_map_x + display_offset;

			let node_y: f64 = px_per_byte * start_addr as f64;
			let node_height: f64 = px_per_byte * (node.size as f64 - 1.0) - node_y;
			let rectangle_y = mem_map_y + node_y;

			let rectangle = Rectangle {
				x: rectangle_x,
				y: rectangle_y,
				width: aperature_width,
				height: node_height,
				color: colour,
			};

			node_vis.rectangle = Some(rectangle);
			apertures.push(node_vis.clone());
		}
	}

	let canvas =
		Canvas::default()
		.block(
			Block::default()
			.borders(Borders::ALL)
			.title(format!(
				"System memory available: {:#010x?} ({} MiB)",
				board.total_system_memory,
				hex_to_mib(board.total_system_memory)
				)
			)
		)
		.paint(|ctx| {
				ctx.draw(&memory_map);

				for aperture in &apertures {

					if aperture.label.is_some() {
						ctx.print(
							aperture.label_x,
							aperture.label_y,
							Span::styled(
								format!("{}",
									aperture.label
										.as_ref()
										.unwrap()
								),
								Style::default()
							)
						);
					}

					if aperture.rectangle.is_none() {
						continue;
					}

					ctx.draw(aperture.rectangle.as_ref().unwrap());
				}

				ctx.print(
					mem_map_x + mem_map_width + 1.25,
					mem_map_y - 0.5,
					Span::styled(format!("{:#010x?}", 0_u64),
					Style::default()),
				);
				ctx.print(
					mem_map_x + mem_map_width + 1.25,
					mem_map_y + mem_map_height,
					Span::styled(format!("{:#010x?}",
							     board.total_system_memory),
					Style::default()),
				);
			}
		)
		.x_bounds([0.0, display_rect.width as f64])
		.y_bounds([0.0, display_rect.height as f64]);

	frame.render_widget(canvas, display_rect);
}

fn format_table_data(board: &mut soc::MPFS) -> (Vec<Vec<String>>, Result<(), ()>)
{
	let mut config_is_valid: Vec<bool> = Vec::new();
	let mut data: Vec<Vec<String>> = Vec::new();

	for memory_aperture in &board.memory_apertures {
		let aperature_start = memory_aperture.get_hw_start_addr(board.total_system_memory);
		let aperature_end = memory_aperture.get_hw_end_addr(board.total_system_memory);

		let mut row_cells: Vec<String> = Vec::new();
		row_cells.push(data.len().to_string());
		row_cells.push(memory_aperture.reg_name.clone());
		row_cells.push(memory_aperture.description.clone());
		row_cells.push(format!("{:#012x?}", memory_aperture.bus_addr));
		row_cells.push(
			format!("{:#08x?}",
				soc::hw_start_addr_to_seg(
					memory_aperture.get_hw_start_addr(u64::MAX).unwrap(),
					memory_aperture.bus_addr)
				)
			);

		if aperature_start.is_err() || aperature_end.is_err() {
			row_cells.push("invalid".to_string());
			row_cells.push("invalid".to_string());
			row_cells.push("n/a MiB".to_string());
			config_is_valid.push(false);
		} else {
			let start = aperature_start.as_ref().unwrap();
			let end = aperature_end.as_ref().unwrap();
			let size = end - start;

			row_cells.push(format!("{:#012x?}", start));
			row_cells.push(format!("{:#012x?}", end));
			row_cells.push(format!("{} MiB", hex_to_mib(size)));
		}

		data.push(row_cells.clone());
	}

	if config_is_valid.len() != board.memory_apertures.len() {
		return (data, Ok(()))
	}
	else {
		return (data, Err(()))
	}
}

fn render_seg_regs<T, G, B: tui::backend::Backend>
(board: &mut soc::MPFS, config_is_valid: Result<T,G>, frame:&mut Frame<B>, display_rect: Rect)
{
	let mut output;

	if config_is_valid.is_ok() {
		output = "seg-reg-config: { ".to_string();
		for memory_aperture in &board.memory_apertures {
			output += &format!(
				"{}: {:#x?}, ",
				memory_aperture.reg_name,
				soc::hw_start_addr_to_seg(memory_aperture.hardware_addr,
							  memory_aperture.bus_addr)
			).to_string();
		}
		output += "}\n";
	} else {
		output = "Cannot calculate seg registers, configuration is invalid as \
			no memory is mapped.".to_string();
	}

	let segs =
		Paragraph::new(output)
		.block(
			Block::default()
			.title("For insertion into config.yaml:")
			.borders(Borders::ALL))
		.style(Style::default());

	frame.render_widget(segs, display_rect);
}

fn render_display<B: tui::backend::Backend>
(board: &mut soc::MPFS, memory_nodes: Option<Vec<MemoryNode>>,
 frame: &mut Frame<B>, display_rect: Rect)
{
	let chunks =
		Layout::default()
		.direction(Direction::Vertical)
		.constraints(
		[
			Constraint::Percentage(90),
			Constraint::Percentage(10),
		]
		.as_ref(),
		)
		.split(display_rect);

	let display_area =
		Layout::default()
		.direction(Direction::Horizontal)
		.constraints(
		[
			Constraint::Percentage(33),
			Constraint::Percentage(67),
		]
		.as_ref(),
		)
		.split(chunks[0]);

	let table_area =
		Layout::default()
		.direction(Direction::Vertical)
		.constraints(
		[
			Constraint::Percentage(60),
			Constraint::Percentage(40),
		]
		.as_ref(),
		)
		.split(display_area[1]);

	let (data, config_is_valid) = format_table_data(board);

	render_seg_regs(board, config_is_valid, frame, chunks[1]);

	render_seg_table(data, frame, table_area[0]);
	render_dt_node_table(board, memory_nodes.clone(), frame, table_area[1]);

	render_visualisation(board, memory_nodes, frame, display_area[0]);
}

fn setup_segs_from_config(board: &mut soc::MPFS, input_file: String)
-> Result<(), Box<dyn std::error::Error>>
{
	let contents = fs::read_to_string(input_file);
	if let Err(error) = &contents {
		return Ok(())
	}

	let d: Value = serde_yaml::from_str(&contents.unwrap())?;
	let seg_config = d["seg-reg-config"].clone();

	let apertures = board.memory_apertures.iter_mut();
	for aperture in apertures {
		let seg_name = aperture.reg_name.as_str();
		let seg_string = seg_config[seg_name].clone();
		if seg_string.as_str().is_some() {
			let seg_string_raw = seg_string.as_str().unwrap();
			let seg_string_trimmed = seg_string_raw.trim_start_matches("0x");
			let seg = u64::from_str_radix(seg_string_trimmed, 16)?;
			aperture.set_hw_start_addr_from_seg(
				board.total_system_memory,
				seg
			)?;
		}
	}
	return Ok(());

}

use std::io::Write;
fn save_segs_to_config(board: &mut soc::MPFS, input_file: String, output_file: String)
-> Result<(), Box<dyn std::error::Error>>
{
	let contents = fs::read_to_string(input_file);
	if let Err(error) = contents {
		return Err(Box::new(error))
	}

	let mut d: Value = serde_yaml::from_str(&contents.unwrap())?;

	for memory_aperture in &board.memory_apertures {
		let seg_value =
			format!("{:#x?}",
				 soc::hw_start_addr_to_seg(memory_aperture.hardware_addr,
							   memory_aperture.bus_addr)
				);
		let seg_as_yaml = Value::String(seg_value);
		d["seg-reg-config"][&memory_aperture.reg_name[..]] = seg_as_yaml;
	}

	let output = serde_yaml::to_string(&d);
	let mut file = fs::File::create(output_file)?;
	file.write_all(output.unwrap()[..].as_bytes())?;

	return Ok(())
}

fn handle_messages(messages: &mut Vec<String>) -> Option<String>
{
	if messages.is_empty(){
		return None;
	}

	let message = messages.pop();
	messages.clear();
	message.as_ref()?;

	let input = message.as_ref().unwrap();
	return Some(input.to_string());
}

/// PolarFire SoC memory aperture configurator
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
	/// input yaml config file
	#[clap(short, long, default_value = "config.yaml")]
	config: String,

	/// input dtb
	#[clap(short, long)]
	dtb: Option<String>,

	/// edit the config in place rather tha use the default output of "generated.yaml"
	#[clap(short, long)]
	in_place: bool,
}
fn main() -> Result<(),Box<dyn std::error::Error>> {
	let args = Args::parse();
	let mut next_state = states::State::default();
	let mut board = soc::MPFS::default();
	let stdout = io::stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut input: String = String::new();
	let mut messages: Vec<String> = Vec::new();
	let input_file = args.config;
	let mut output_file = "generated.yaml".to_string();
	let mut memory_nodes: Option<Vec<MemoryNode>> = None;
	if args.in_place {
		output_file = input_file.clone();
	}

	if args.dtb.is_some() {
		let dtb_file = args.dtb.unwrap();
		let mut dtb_handle = fs::File::open(dtb_file)?;
		let mut dtb = Vec::new();
		dtb_handle.read_to_end(&mut dtb)?;
		let dt = device_tree::DeviceTree::load(dtb.as_slice())
				.or(Err("bad dtb"))?;
		let root_node = dt.root;
		memory_nodes = Some(dt::get_memory_nodes(root_node)?);
	}

	setup_segs_from_config(&mut board, input_file.clone())?;

	terminal.clear()?;
	enable_raw_mode()?;
	terminal.clear()?;

	loop {
		let command_text = next_state.command_text.clone();
		terminal.draw(|frame| {
			let entire_window =
				Layout::default()
				.direction(Direction::Vertical)
				.constraints(
				[
					Constraint::Percentage(90),
					Constraint::Percentage(10),
				]
				.as_ref(),
				)
				.split(frame.size());

			render_display(&mut board, memory_nodes.clone(), frame, entire_window[0]);

			let txt = format!("{}\n{}", command_text, input);

			let graph =
				Paragraph::new(txt)
				.block(
					Block::default()
					.title("Press Esc to quit, enter \"save\" to save.")
					.borders(Borders::ALL))
				.style(Style::default());

			frame.render_widget(graph, entire_window[1]);
		})?;

		if event::poll(Duration::from_millis(30))? {
			if let Event::Key(key) = event::read()? {
				match key.code {
					KeyCode::Char(c) => {
						input.push(c);
					}
					KeyCode::Backspace => {
						input.pop();
					}
					KeyCode::Esc => {
						terminal.clear()?;
						if disable_raw_mode().is_err() {
							panic!("Failed to clean up terminal");
						}
						return Ok(());
					}
					KeyCode::Enter => {
						messages.push(input.drain(..).collect());
					}
					_ => {}
				}
			}
		}

		let input = handle_messages(&mut messages);
		if let Some(command) = input.clone() {
			if command.contains("save") {
				save_segs_to_config(&mut board, input_file.clone(), output_file.clone())?;
				continue;
			}
		}
		next_state = states::get_next_state(next_state, &mut board, input);

	}
}
