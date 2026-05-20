use std::{
	cmp::Ordering,
	fs::{self, DirEntry},
	path::PathBuf,
};

use eframe::{
	CreationContext,
	egui::{CollapsingHeader, Ui},
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
		children: Vec<FileNode>,
		is_open: bool,
	},
	File {
		name: String,
	},
	Unknown {
		name: String,
	},
}

impl FileNode {
	pub fn directory(path: PathBuf) -> Self {
		FileNode::Directory {
			path,
			children: vec![],
			is_open: false,
		}
	}

	fn dir_entry(entry: DirEntry) -> Self {
		let file_type = entry.file_type().expect("file type should always be valid");

		if file_type.is_dir() {
			FileNode::directory(entry.path())
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
			FileNode::Directory {
				path,
				children,
				is_open,
			} => {
				*is_open = true;
				if !has_been_opened {
					*children = fs::read_dir(&path)
						.expect("path must point to directory")
						.map(|f| FileNode::dir_entry(f.unwrap()))
						.collect();
					children.sort_by(|a, b| match (a, b) {
						(FileNode::Directory { .. }, FileNode::Directory { .. }) => a.name().cmp(b.name()),
						(FileNode::Directory { .. }, _) => Ordering::Less,
						(_, FileNode::Directory { .. }) => Ordering::Greater,
						(_, _) => a.name().cmp(b.name()),
					});
				}
			}
			FileNode::Unknown { .. } => {}
			FileNode::File { .. } => {}
		}
	}

	pub fn close(&mut self) {
		match self {
			FileNode::Directory { is_open, .. } => *is_open = false,
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

	pub fn is_open(&self) -> bool {
		match self {
			FileNode::Directory { is_open, .. } => *is_open,
			FileNode::Unknown { .. } => false,
			FileNode::File { .. } => false,
		}
	}

	pub fn show(&mut self, ui: &mut Ui) {
		let name = self.name().to_owned();
		let is_open = self.is_open();
		let has_been_opened = self.has_been_opened();
		match self {
			FileNode::Directory { children, .. } => {
				let header = ui.collapsing(name, |ui| {
					for child in children {
						child.show(ui);
					}
				});
				if header.header_response.clicked() && header.fully_open() {
					self.open();
				}
			}
			FileNode::File { .. } => {
				let header = CollapsingHeader::new(name)
					.open(if !has_been_opened { Some(false) } else { None })
					.show(ui, |ui| {
						ui.label("mraow");
					});
				if !has_been_opened {
					if header.header_response.double_clicked() {
						self.open();
					}
				}
			}
			FileNode::Unknown { .. } => {
				ui.label(name);
			}
		}
	}
}

pub struct State {}

impl State {
	pub fn new(_ctx: &CreationContext) -> Self {
		Self {}
	}

	fn file_tree_ui(&mut self, ui: &mut Ui, root: &mut FileNode) {
		root.show(ui);
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
