// SPDX-License-Identifier: MIT or GPL-2.0

#![allow(unused_variables)]
#![allow(dead_code)]
#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use std::io;
use std::time::Duration;
use std::fs;
use tui::{
	backend::{CrosstermBackend},
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	widgets::{Block, Borders, Paragraph, Cell, Row, Table},
	widgets::canvas::{Canvas, Rectangle},
	Frame, Terminal,
	text::Span
};
use crossterm::{
	event::{self, Event, KeyCode},
	terminal::{disable_raw_mode, enable_raw_mode},
};
use yaml_rust::yaml::YamlLoader;

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

fn render_table<'a, B: tui::backend::Backend>
(data: Vec<Vec<String>>, frame:&mut Frame<B>, display_rect: Rect)
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

	frame.render_widget(table, display_rect);
}

fn render_visualisation<B: tui::backend::Backend>
(board: &mut soc::MPFS, frame:&mut Frame<B>, display_rect: Rect)
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
	let mut aperatures: Vec<Rectangle> = Vec::new();
	let num_apertures = 6.0;
	let aperature_width = mem_map_width / (num_apertures + 1.0);
	let mut display_offset = aperature_width / num_apertures;
	for aperature in memory_apertures {
		let aperature_start = aperature.get_hw_start_addr(board.total_system_memory);
		let aperature_end = aperature.get_hw_end_addr(board.total_system_memory);
		let colour = *aperature_colours.next().unwrap(); // yeah, yeah this could crash

		if aperature_start.is_ok() && aperature_end.is_ok() {
			let aperture_y: f64 = px_per_byte * aperature_start.unwrap() as f64;
			let aperture_height: f64 = px_per_byte * aperature_end.unwrap() as f64
						   - aperture_y;
			let rectangle = Rectangle {
				x: mem_map_x + display_offset,
				y: mem_map_y + aperture_y,
				width: aperature_width,
				height: aperture_height,
				color: colour,
			};
			aperatures.push(rectangle.clone());
		}
		display_offset += aperature_width + aperature_width / num_apertures;
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

				let mut remove_me: u64 = 0;
				for rectangle in &aperatures {
					ctx.draw(rectangle);
					ctx.print(
						rectangle.x + 0.5 * rectangle.width,
						mem_map_y - 0.5,
						Span::styled(format!("{}", remove_me),
						Style::default().fg(Color::White)),
					);
					remove_me += 1;
				}
				ctx.print(
					mem_map_x + mem_map_width + 1.25,
					mem_map_y - 0.5,
					Span::styled(format!("{:#010x?}", 0_u64),
					Style::default().fg(Color::White)),
				);
				ctx.print(
					mem_map_x + mem_map_width + 1.25,
					mem_map_y + mem_map_height,
					Span::styled(format!("{:#010x?}",
							     board.total_system_memory),
					Style::default().fg(Color::White)),
				);
			}
		)
		.x_bounds([0.0, display_rect.width as f64])
		.y_bounds([0.0, display_rect.height as f64]);

	frame.render_widget(canvas, display_rect);
}

fn format_table_data(board: &mut soc::MPFS) -> (Vec<Vec<String>>, Result<(), ()>)
{
	let mut config_is_valid: bool = true;
	let mut data: Vec<Vec<String>> = Vec::new();

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

	if config_is_valid {
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

	frame.render_widget(segs, display_rect);
}

fn render_display<B: tui::backend::Backend>
(board: &mut soc::MPFS, frame:&mut Frame<B>, display_rect: Rect)
{
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

	let (data, config_is_valid) = format_table_data(board);

	render_seg_regs(board, config_is_valid, frame, chunks[1]);

	render_table(data, frame, display_area[1]);

	render_visualisation(board, frame, display_area[0]);
}

fn setup_segs_from_config(board: &mut soc::MPFS, doc: &yaml_rust::yaml::Yaml)
// -> Result<(), Box<dyn std::error::Error>>
{
	let seg_config = &doc["seg-reg-config"];
	if *seg_config != yaml_rust::yaml::Yaml::BadValue {
		let apertures = board.memory_apertures.iter_mut();
		for aperture in apertures {
			let seg_name = aperture.reg_name.as_str();
			let seg_string = seg_config[seg_name].clone();
			println!("{:?}", seg_string.as_str().unwrap());
			let seg_string_raw = seg_string.as_str().unwrap();
			let seg_string_trimmed = seg_string_raw.trim_start_matches("0x");
			let seg = u64::from_str_radix(seg_string_trimmed, 16);
			if seg.is_ok(){
				aperture.set_hw_start_addr_from_seg(board.total_system_memory, seg.unwrap());
			}
		}
	}
}
fn main() -> Result<(), io::Error> {
	let mut next_state = states::State::default();
	let mut board = soc::MPFS::default();
	let stdout = io::stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut input: String = String::new();
	let mut messages: Vec<String> = Vec::new();
	let filename = "config.yaml";

	let contents = fs::read_to_string(filename);
	if contents.is_ok() {
		let doc = &YamlLoader::load_from_str(&contents.unwrap()).unwrap()[0];
		setup_segs_from_config(&mut board, doc);
	}

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
					Constraint::Percentage(85),
					Constraint::Percentage(15),
				]
				.as_ref(),
				)
				.split(frame.size());
			
			render_display(&mut board, frame, entire_window[0]);

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
