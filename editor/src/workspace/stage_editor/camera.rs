use glam::{Mat4, Quat, Vec2, Vec3};

use crate::workspace::stage_editor::Actions;

pub struct ViewCamera {
	position: Vec3,
	rotation: Vec2,
}

impl ViewCamera {
	pub fn new() -> Self {
		ViewCamera {
			position: Vec3::ZERO,
			rotation: Vec2::ZERO,
		}
	}

	pub fn update(&mut self, actions: &Actions) {
		let x_axis = actions.held(super::StageAction::MoveRight) as i8
			- actions.held(super::StageAction::MoveLeft) as i8;
		let y_axis = actions.held(super::StageAction::MoveDown) as i8
			- actions.held(super::StageAction::MoveUp) as i8;
		let z_axis = actions.held(super::StageAction::MoveForward) as i8
			- actions.held(super::StageAction::MoveBack) as i8;

		if actions.held(super::StageAction::Lock) {
			self.rotation += actions.mouse_delta * 0.016;
		}
		self.rotation.y = self
			.rotation
			.y
			.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
		let x_rot = Quat::from_axis_angle(Vec3::Y, self.rotation.x);
		let y_rot = Quat::from_axis_angle(Vec3::NEG_X, self.rotation.y);

		let movement_direction =
			(x_rot * y_rot).mul_vec3(Vec3::new(-x_axis as f32, 0.0, z_axis as f32))
				+ Vec3::Y * y_axis as f32;

		self.position += movement_direction * 10.0;
	}

	pub fn calculate_matrix(&self) -> Mat4 {
		let up = Vec3::Y;
		let x_rot = Quat::from_axis_angle(up, self.rotation.x);
		let y_rot = Quat::from_axis_angle(Vec3::NEG_X, self.rotation.y);
		glam::camera::rh::view::look_to_mat4(self.position, (x_rot * y_rot).mul_vec3(Vec3::Z), up)
	}
}
