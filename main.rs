// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use std::io;
use std::time::Duration;
use tui::{
	backend::{CrosstermBackend},
	layout::{Constraint, Direction, Layout, Rect},
	text::Spans,
	style::{Color, Modifier, Style},
	widgets::{Block, Borders, Paragraph, Cell, Row, Table, TableState},
	Frame, Terminal,
};
use crossterm::{
	event::{self, Event, KeyCode},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode},
};

mod soc;
use crate::soc::SoC;
use crate::soc::Aperture;

#[derive(Clone)]
struct State {
	state_id: States,
	previous_state_id: States,
	command_text: String
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Debug)]
enum States {
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

	let mut next_state = State {
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

fn get_next_state(current_state: State, board: &mut soc::MPFS, input: &mut Vec<String>) -> State 
{
	let state_id = current_state.state_id as usize;
	let next_state = STATE_HANDLERS[state_id](current_state, board, input.pop());
	input.clear();

	return next_state
}

fn hex_to_mib(hex: u64) -> u64
{
	return hex / (2_u64.pow(10).pow(2))
}

fn display_status<'a, B: tui::backend::Backend>(board: &mut soc::MPFS, frame:&mut Frame<B>, table_display: Rect, segs_display: Rect)
{
	let selected_style = Style::default().add_modifier(Modifier::REVERSED);
	let normal_style = Style::default().bg(Color::Blue);
	let header_cells =
		[
			"ID", "Register", "Description", "Bus Address",
			"Aperture HW Start", "Aperture HW End", "Aperature Size",
		 ]
		.iter()
		.map(|h|
			Cell::from(*h)
			.style(
				Style::default()
				.fg(Color::White)
				.bg(Color::Black)
			)
		);
	let header =
		Row::new(header_cells).height(1).bottom_margin(1);
	
	let mut data: Vec<Vec<String>> = Vec::new();
	let mut config_is_valid: bool = true;

	for memory_aperture in &board.memory_apertures {
		let aperature_start = memory_aperture.get_hw_start_addr(board.total_system_memory);
		let aperature_end = memory_aperture.get_hw_end_addr(board.total_system_memory);
	
		let mut row_cells: Vec<String> = Vec::new();
		row_cells.push(data.len().to_string());
		row_cells.push(memory_aperture.reg_name.clone());
		row_cells.push(memory_aperture.description.clone());
		row_cells.push(format!("{:#012x?}", memory_aperture.bus_addr));

		if aperature_start.is_err() || aperature_end.is_err() {
			row_cells.push("invalid".to_string());
			row_cells.push("invalid".to_string());
			row_cells.push("n/a MiB".to_string());
			config_is_valid = false;
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

	let rows = data.iter().map(|item| {
		let cells = item.iter().map(|c|
			Cell::from(c.clone())
		);
		Row::new(cells).height(1).bottom_margin(1)
	});

	let mut output = "".to_string();
	if config_is_valid {
		output = format!("seg-reg-config: {{ ").to_owned();
		for memory_aperture in &board.memory_apertures {
			output += &format!(
				"{}: {:#x?}, ",
				memory_aperture.reg_name,
				soc::hw_start_addr_to_seg(memory_aperture.hardware_addr,
							  memory_aperture.bus_addr)
			).to_string();
		}
		output += &format!("}}\n").to_string();
	} else {
		output = format!("Cannot calculate seg registers, configuration is invalid.");
	}
	let segs =
		Paragraph::new(output)
		.block(
			Block::default()
			.title("For insertion into config.yaml:")
			.borders(Borders::ALL))
		.style(
			Style::default()
			.fg(Color::White)
			.bg(Color::Black)
		);

	let table =
		Table::new(rows)
		.header(header)
		.block(
			Block::default()
			.borders(Borders::ALL)
			.title(format!(
				"System memory available: {:#012x?}",
				board.total_system_memory)
			)
		)
		.style(
			Style::default()
			.fg(Color::White)
			.bg(Color::Black)
		)
		.highlight_style(selected_style)
		.highlight_symbol(">> ")
		.widths(&[
			Constraint::Percentage(5),
			Constraint::Percentage(5),
			Constraint::Percentage(25),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
		]);

	frame.render_widget(table, table_display);
	frame.render_widget(segs, segs_display);
}

fn main() -> Result<(), io::Error> {
	let mut next_state = State {
		state_id: States::Init,
		previous_state_id: States::Exit,
		command_text: "Press Enter to begin...".to_string()
	};
	let mut board = soc::MPFS::default();
	let mut stdout = io::stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut input: String = String::new();
	let mut messages: Vec<String> = Vec::new();

	enable_raw_mode()?;
	terminal.clear()?;

	loop {
		let command_text = next_state.command_text.clone();
		terminal.draw(|frame| {
			let chunks = 
				Layout::default()
				.direction(Direction::Vertical)
				.constraints(
				[
					Constraint::Percentage(70),
					Constraint::Percentage(15),
					Constraint::Percentage(15),
				]
				.as_ref(),
				)
				.split(frame.size());

			display_status(&mut board, frame, chunks[0], chunks[1]);
			
			let txt = format!("{}\n{}", command_text, input);
			
			let graph =
				Paragraph::new(txt)
				.block(
					Block::default()
					.title("Press Esc to quit")
					.borders(Borders::ALL))
				.style(
					Style::default()
					.fg(Color::White)
					.bg(Color::Black)
				);

			frame.render_widget(graph, chunks[2]);
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
						disable_raw_mode();
						return Ok(());
					}
					KeyCode::Enter => {
						messages.push(input.drain(..).collect());
					}
					_ => {}
				}
			}
		}

		next_state = get_next_state(next_state, &mut board, &mut messages);
	}
}
