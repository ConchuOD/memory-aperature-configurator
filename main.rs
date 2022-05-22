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
	style::{Color, Modifier, Style},
	widgets::{Block, Borders, Paragraph, Cell, Row, Table},
	Frame, Terminal,
};
use crossterm::{
	event::{self, Event, KeyCode},
	terminal::{disable_raw_mode, enable_raw_mode},
};

mod soc;
use crate::soc::Aperture;
mod states;

fn hex_to_mib(hex: u64) -> u64
{
	return hex / (2_u64.pow(10).pow(2))
}

fn display_status<'a, B: tui::backend::Backend>
(board: &mut soc::MPFS, frame:&mut Frame<B>, display_rect: Rect)
{
	let selected_style = Style::default().add_modifier(Modifier::REVERSED);
	let normal_style = Style::default().bg(Color::Blue);
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
			row_cells.push("invalid".to_string());
			row_cells.push("n/a MiB".to_string());
			config_is_valid = false;
		} else {
			let start = aperature_start.as_ref().unwrap();
			let end = aperature_end.as_ref().unwrap();
			let size = end - start;

			row_cells.push(
				format!("{:#04x?}",
					soc::hw_start_addr_to_seg(*start,
					memory_aperture.bus_addr)
					)
				);
			row_cells.push(format!("{:#012x?}", start));
			row_cells.push(format!("{:#012x?}", end));
			row_cells.push(format!("{} MiB", hex_to_mib(size)));
		}

		data.push(row_cells.clone());
	}

	let rows = data.iter().map(|item| {
		let cells = item.iter().map(|c|
			return Cell::from(c.clone())
		);
		return Row::new(cells).height(1).bottom_margin(1)
	});

	let mut output;
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

	let chunks =
		Layout::default()
		.direction(Direction::Vertical)
		.constraints(
		[
			Constraint::Percentage(85),
			Constraint::Percentage(15),
		]
		.as_ref(),
		)
		.split(display_rect);

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
				"System memory available: {:#012x?} ({} MiB)",
				board.total_system_memory,
				hex_to_mib(board.total_system_memory)
				)
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
			Constraint::Percentage(10),
			Constraint::Percentage(15),
			Constraint::Percentage(12),
			Constraint::Percentage(8),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
			Constraint::Percentage(12),
		]);

	frame.render_widget(table, chunks[0]);
	frame.render_widget(segs, chunks[1]);
}

fn main() -> Result<(), io::Error> {
	let mut next_state = states::State::default();
	let mut board = soc::MPFS::default();
	let stdout = io::stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut input: String = String::new();
	let mut messages: Vec<String> = Vec::new();

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
					Constraint::Percentage(85),
					Constraint::Percentage(15),
				]
				.as_ref(),
				)
				.split(frame.size());
			
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
				.split(entire_window[0]);

			display_status(&mut board, frame, display_area[1]);
			
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

		next_state = states::get_next_state(next_state, &mut board, &mut messages);
	}
}
