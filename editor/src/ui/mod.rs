mod file_tree;

use std::{
	cmp::Ordering,
	fs::{self, DirEntry},
	path::PathBuf,
};

use eframe::{
	CreationContext,
	egui::{self, CollapsingHeader, Ui},
};
use egui_dock::TabViewer;

pub enum TabType {
	FileTree { root: FileNode },
}

impl TabType {
	fn name(&self) -> &str {
		match self {
			TabType::FileTree { .. } => "File Tree",
		}
	}
}

pub enum FileNode {
	Directory {
		path: PathBuf,
		children: Vec<(FileNode, bool)>,
	},
	File {
		name: String,
	},
	Unknown {
		name: String,
	},
}

impl FileNode {
	pub fn new_directory(path: PathBuf) -> Self {
		FileNode::Directory {
			path,
			children: vec![],
		}
	}

	fn dir_entry(entry: DirEntry) -> Self {
		let file_type = entry.file_type().expect("file type should always be valid");

		if file_type.is_dir() {
			FileNode::new_directory(entry.path())
		} else if file_type.is_file() {
			let name = entry
				.file_name()
				.to_str()
				.expect("path should be valid UTF-8")
				.to_owned();

			FileNode::File { name }
		} else {
			FileNode::Unknown {
				name: entry
					.file_name()
					.to_str()
					.expect("path should be valid UTF-8")
					.to_owned(),
			}
		}
	}

	pub fn name(&self) -> &str {
		match self {
			FileNode::Directory { path, .. } => path
				.file_name()
				.expect("path should be canonicalised")
				.to_str()
				.expect("path should be UTF-8 compatible"),
			FileNode::Unknown { name } => &name,
			FileNode::File { name } => &name,
		}
	}

	pub fn open(&mut self) {
		let has_been_opened = self.has_been_opened();
		match self {
			FileNode::Directory { path, children } => {
				if !has_been_opened {
					*children = fs::read_dir(&path)
						.expect("path must point to directory")
						.map(|f| (FileNode::dir_entry(f.unwrap()), false))
						.collect();
					children.sort_by(|a, b| match (&a.0, &b.0) {
						(FileNode::Directory { .. }, FileNode::Directory { .. }) => a.0.name().cmp(b.0.name()),
						(FileNode::Directory { .. }, _) => Ordering::Less,
						(_, FileNode::Directory { .. }) => Ordering::Greater,
						(_, _) => a.0.name().cmp(b.0.name()),
					});
				}
			}
			FileNode::Unknown { .. } => {}
			FileNode::File { .. } => {}
		}
	}

	pub fn has_been_opened(&self) -> bool {
		match self {
			FileNode::Directory { children, .. } => !children.is_empty(),
			FileNode::Unknown { .. } => false,
			FileNode::File { .. } => false,
		}
	}

	pub fn show(&mut self, ui: &mut Ui, is_open: &mut bool) {
		let has_been_opened = self.has_been_opened();
		match self {
			FileNode::Directory { children, path, .. } => {
				let name = path
					.file_name()
					.expect("path should be canonicalised")
					.to_str()
					.expect("path should be UTF-8 compatible");
				let header = CollapsingHeader::new(name)
					.open(Some(*is_open))
					.show(ui, |ui| {
						for (child, is_open) in children {
							child.show(ui, is_open);
						}
					});
				if header.header_response.clicked() {
					*is_open = !*is_open;
					self.open();
				}
			}
			FileNode::File { .. } => {
				ui.scope(|ui| {
					let name = self.name();
					ui.multiply_opacity(0.7);
					let header = CollapsingHeader::new(name)
						.icon(circle_icon)
						.open(if !has_been_opened { Some(false) } else { None })
						.show(ui, |ui| {
							ui.label("mraow");
						});
					if !has_been_opened {
						if header.header_response.double_clicked() {
							*is_open = !*is_open;
							self.open();
						}
					}
				});
			}
			FileNode::Unknown { .. } => {
				let name = self.name();
				ui.label(name);
			}
		}
	}
}

fn circle_icon(ui: &mut Ui, openness: f32, response: &egui::Response) {
	let stroke = ui.style().interact(&response).fg_stroke;
	let radius = egui::lerp(2.0..=3.0, openness);
	ui.painter()
		.circle_filled(response.rect.center(), radius, stroke.color);
}

pub struct State {}

impl State {
	pub fn new(_ctx: &CreationContext) -> Self {
		Self {}
	}

	fn file_tree_ui(&mut self, ui: &mut Ui, root: &mut FileNode) {
		root.open();
		root.show(ui, &mut true);
	}
}

impl TabViewer for State {
	type Tab = TabType;

	fn title(&mut self, tab: &mut Self::Tab) -> eframe::egui::WidgetText {
		tab.name().into()
	}

	fn ui(&mut self, ui: &mut eframe::egui::Ui, tab: &mut Self::Tab) {
		match tab {
			TabType::FileTree { root } => self.file_tree_ui(ui, root),
		};
	}
}
