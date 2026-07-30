use std::path::PathBuf;

use eframe::egui::{Response, Ui};

pub trait Node {
  fn name(&self) -> &str;
  fn show(ui: &mut Ui) -> Response;
}

pub struct Directory {
  path: PathBuf,
  
}

struct FileTree {
  children: Node
}

pub fn file_tree() {}
