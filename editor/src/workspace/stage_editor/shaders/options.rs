use std::{fmt, hash::Hasher};

use phf_shared::{FmtConst, PhfBorrow, PhfHash};
use zerocopy::{ByteHash, Immutable, IntoBytes};

#[derive(Clone, Copy, ByteHash, Debug, Immutable, IntoBytes, PartialEq, Eq)]
#[repr(C)]
pub struct ShaderOptions {
	pub textured: bool,
	pub vertex_colours: bool,
}
impl PhfHash for ShaderOptions {
	#[inline]
	fn phf_hash<H: Hasher>(&self, state: &mut H) {
		u8::phf_hash_slice(self.as_bytes(), state);
	}
}
impl FmtConst for ShaderOptions {
	/// Print a `const` expression representing this value.
	fn fmt_const(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		std::fmt::Debug::fmt(self, f)
	}
}
impl PhfBorrow<ShaderOptions> for ShaderOptions {
	/// Convert a reference to `self` to a reference to the borrowed type.
	fn borrow(&self) -> &Self {
		self
	}
}
