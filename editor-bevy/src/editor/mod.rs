mod byml;
mod detect_format;
mod menu;
mod project_explorer;
mod sarc;
mod stage;

use std::{
	any::type_name_of_val,
	fs::{self, File},
	path::PathBuf,
	thread::{self, JoinHandle},
};

use bevy::{prelude::*, tasks::*};
use bevy_egui::{
	EguiContexts, EguiPrimaryContextPass,
	egui::{self, CentralPanel},
};
use cassava_project::ALProject;
use egui_dock::{DockArea, DockState, SurfaceIndex, TabViewer};
use egui_toast::{Toast, ToastKind, Toasts};
use rfd::AsyncFileDialog;

use crate::{
	Project, UiStates,
	config::Config,
	editor::{
		byml::BymlEditorTab,
		detect_format::{Format, detect_format},
		sarc::SarcEditorTab,
		stage::StageEditorTab,
	},
};

pub fn plugin(app: &mut App) {
	app
		.insert_resource(Editor {
			dock_state: DockState::new(vec![]),
			toasts: Toasts::new(),
		})
		.add_message::<OpenFile>()
		.add_message::<NewFileTab>()
		.add_systems(
			Update,
			(
				delete_standalone_entities,
				add_project_explorer,
				// try_dnd_tests,
				rfd_shortcut_task,
				open_files,
				add_tabs,
			),
		)
		.add_systems(OnEnter(UiStates::Editor), on_editor_start)
		.add_systems(
			EguiPrimaryContextPass,
			(setup_viewport_ui, tick_editor.after(setup_viewport_ui)),
		);
}

#[derive(Component, Clone, Default)]
struct StandaloneOnly;
fn on_editor_start(mut commands: Commands) {
	commands.spawn_scene(bsn! {
		Node {
			margin: px(10),
			align_self: AlignSelf::Center,
			justify_self: JustifySelf::Center,
		}
		Text::new("To get started, drag a file\nonto this window, or press Ctrl+O.")
		StandaloneOnly
	});
}

fn delete_standalone_entities(
	mut commands: Commands,
	query: Query<Entity, With<StandaloneOnly>>,
	_: Single<(), With<Project>>,
) {
	for entity in query {
		commands.entity(entity).despawn();
	}
}

struct ProjectExplorerTab;
impl ProjectExplorerTab {
	pub fn show(&mut self, _ui: &mut egui::Ui) {}
}

enum Tabs {
	Explorer(ProjectExplorerTab),
	Stage(StageEditorTab),
	Sarc(SarcEditorTab),
	Byml(BymlEditorTab),
	Bntx,
	Bfres,
	Bnsh,
	Bffnt,
	Bflyt,
	Kcl,
}

impl Tabs {
	pub fn new_format(format: Format, name: String, data: Vec<u8>) -> Result<Self> {
		match format {
			Format::Szs | Format::Sarc => Ok(Self::Sarc(SarcEditorTab::new(
				name,
				data,
				format == Format::Szs,
			)?)),
			Format::Byml => Ok(Self::Byml(BymlEditorTab::new(name, data)?)),
			Format::Bntx => todo!(),
			Format::Kcl => todo!(),
		}
	}
}

struct TabContext<'a> {
	commands: Commands<'a, 'a>,
	toasts: &'a mut Toasts,
	project: Option<&'a mut ALProject>,
}

impl<'a> TabViewer for TabContext<'a> {
	type Tab = Tabs;

	fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
		match tab {
			Tabs::Explorer(_) => "Project Explorer".into(),
			Tabs::Stage(tab) => tab.title(),
			Tabs::Sarc(tab) => tab.title(),
			Tabs::Byml(tab) => tab.title(),
			Tabs::Bntx => todo!(),
			Tabs::Bfres => todo!(),
			Tabs::Bnsh => todo!(),
			Tabs::Bffnt => todo!(),
			Tabs::Bflyt => todo!(),
			Tabs::Kcl => todo!(),
		}
	}

	fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
		match tab {
			Tabs::Explorer(tab) => tab.show(ui),
			Tabs::Stage(tab) => tab.show(ui),
			Tabs::Sarc(tab) => tab.show(ui, self),
			Tabs::Byml(tab) => tab.show(ui),
			_ => todo!("ui for {}", type_name_of_val(&tab)),
		}
	}
	fn on_tab_button(&mut self, _tab: &mut Self::Tab, _response: &egui::Response) {}

	fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
		false
	}

	fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
		false
	}
}

#[derive(Resource)]
struct Editor {
	dock_state: DockState<Tabs>,
	toasts: Toasts,
}

fn add_project_explorer(mut editor: ResMut<Editor>, _query: Single<(), Added<Project>>) {
	editor
		.dock_state
		.main_surface_mut()
		.push_to_first_leaf(Tabs::Explorer(ProjectExplorerTab));
}

#[derive(Message)]
struct OpenFile(PathBuf);

fn rfd_shortcut_task(
	input: Res<ButtonInput<KeyCode>>,
	mut open_file_task: Local<Option<Task<Option<rfd::FileHandle>>>>,
	mut open_task_writer: MessageWriter<OpenFile>,
) {
	match &mut *open_file_task {
		Some(task_ref) => {
			let completed = block_on(poll_once(task_ref));
			if let Some(maybe_file) = completed {
				open_file_task.take();
				if let Some(file) = maybe_file {
					open_task_writer.write(OpenFile(file.path().to_path_buf()));
				}
			}
		}
		None => {}
	}
	if input.pressed(KeyCode::ControlLeft) && input.just_pressed(KeyCode::KeyO) {
		println!("Open time");
		*open_file_task = Some(AsyncComputeTaskPool::get().spawn(AsyncFileDialog::new().pick_file()));
	}
}

fn open_files(
	mut editor: ResMut<Editor>,
	mut active_tasks: Local<Vec<Option<JoinHandle<(String, std::io::Result<Vec<u8>>)>>>>,
	mut new_tasks: MessageReader<OpenFile>,
) {
	for task in new_tasks.read() {
		let path = task.0.clone();
		active_tasks.push(Some(thread::spawn(move || {
			let data = fs::read(&path);
			(
				path
					.file_name()
					.expect("no file name??")
					.to_string_lossy()
					.into_owned(),
				data,
			)
		})))
	}
	active_tasks.retain_mut(|el| {
		if let Some(task) = el.take_if(|task| task.is_finished()) {
			// todo: handle errors when reading the file
			let (name, data) = task.join().expect("read tasks should never fail");
			let data = match data {
				Ok(data) => data,
				Err(err) => {
					editor.toasts.add(
						Toast::new()
							.kind(ToastKind::Info)
							.text(format!("{name}\n{err}",)),
					);
					error!("failed to read file \"{name}\": {err}");
					return false;
				}
			};
			add_file_tab(data, name, &mut editor);
			false
		} else {
			true
		}
	});
}

#[derive(Message)]
struct NewFileTab {
	name: String,
	data: Vec<u8>,
}

fn add_tabs(mut editor: ResMut<Editor>, mut message: ResMut<Messages<NewFileTab>>) {
	for task in message.drain() {
		add_file_tab(task.data, task.name, &mut editor);
	}
}

fn add_file_tab(data: Vec<u8>, name: String, editor: &mut Editor) {
	let Some(format) = detect_format(&data) else {
		editor.toasts.add(
			Toast::new()
				.kind(ToastKind::Info)
				.text(format!("{name}\nUnsupported file format")),
		);
		return;
	};
	match Tabs::new_format(format, name.clone(), data) {
		Ok(tab) => {
			editor.dock_state.add_window(vec![tab]);
		}
		Err(err) => {
			editor.toasts.add(
				Toast::new()
					.kind(ToastKind::Info)
					.text(format!("{name}\nFailed to open window: {err}")),
			);
		}
	}
}

#[derive(Resource)]
struct ViewportUi(egui::Ui);
fn setup_viewport_ui(mut commands: Commands, mut contexts: EguiContexts) -> Result {
	let ctx = contexts.ctx_mut()?;
	let mut viewport_ui = egui::Ui::new(
		ctx.clone(),
		"editor viewport".into(),
		egui::UiBuilder::new()
			.layer_id(egui::LayerId::background())
			.max_rect(ctx.viewport_rect()),
	);
	commands.insert_resource(ViewportUi(viewport_ui));
	Ok(())
}

fn tick_editor(
	mut contexts: EguiContexts,
	mut editor: ResMut<Editor>,
	commands: Commands,
	config: Res<Config>,
	mut project: Option<ResMut<Project>>,
	mut viewport_ui: ResMut<ViewportUi>,
) -> Result {
	let mut project = project.as_deref_mut().map(|project| &mut project.0);
	let Editor { dock_state, toasts } = &mut *editor;
	DockArea::new(dock_state).show_inside(
		&mut viewport_ui.0,
		&mut TabContext {
			commands,
			toasts,
			project,
		},
	);

	Ok(())
}
