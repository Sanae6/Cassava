use std::{any::type_name, error::Error, fmt, fs, path::Path};

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever, whatever};

pub trait ProjectAsset: Sized {
	fn load(path: &Path) -> Result<Self, Whatever>;
	fn save(&self, path: &Path) -> Result<(), Whatever>;
}

pub struct DiskRef<T: ProjectAsset> {
	pub(crate) path: RelativePathBuf,
	pub(crate) value: Option<T>,
}

impl<T: ProjectAsset> DiskRef<T> {
	pub fn load(&mut self, root_dir: &Path) -> Result<&mut T, Whatever> {
		match &mut self.value {
			Some(value) => Ok(value),
			opt @ None => Ok(opt.insert(T::load(&self.path.to_logical_path(root_dir))?)),
		}
	}

	pub fn save(&self, root_dir: &Path) -> Result<(), Whatever> {
		match &self.value {
			Some(value) => value.save(&root_dir.join(self.path.to_logical_path(&root_dir))),
			None => {
				tracing::warn!("{} was not loaded, not attempting save", self.path);
				Ok(())
			}
		}
	}
}

impl<T: fmt::Debug + ProjectAsset> fmt::Debug for DiskRef<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.value {
			Some(value) => {
				f.write_str("<")?;
				self.path.fmt(f)?;
				f.write_str("> ")?;
				value.fmt(f)
			}
			None => {
				f.write_str("<unloaded ")?;
				f.write_str(type_name::<T>())?;
				f.write_str(" at ")?;
				self.path.fmt(f)?;
				f.write_str(">")
			}
		}
	}
}

impl<T: ProjectAsset> Serialize for DiskRef<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.path.serialize(serializer)
	}
}

impl<'de, T: ProjectAsset> Deserialize<'de> for DiskRef<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(Self {
			path: Deserialize::deserialize(deserializer)?,
			value: None,
		})
	}
}

impl ProjectAsset for Vec<u8> {
	fn load(path: &Path) -> Result<Self, Whatever> {
		fs::read(path).whatever_context("failed to read data")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		fs::write(path, self).whatever_context("failed to write data")
	}
}
