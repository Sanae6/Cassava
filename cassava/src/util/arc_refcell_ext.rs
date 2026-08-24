use std::{
	any::Any,
	ops::{Deref, DerefMut},
	sync::Arc,
};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

pub struct ArcAtomicRef<T: ?Sized + 'static> {
	#[allow(unused)]
	arc: Arc<dyn Any>,
	borrow: AtomicRef<'static, T>,
}

impl<T> Deref for ArcAtomicRef<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.borrow
	}
}

impl<T> ArcAtomicRef<T> {
	pub fn map<U: ?Sized>(orig: ArcAtomicRef<T>, func: impl FnOnce(&T) -> &U) -> ArcAtomicRef<U> {
		ArcAtomicRef {
			arc: orig.arc,
			borrow: AtomicRef::map(orig.borrow, func),
		}
	}

	pub fn filter_map<U: ?Sized>(
		orig: ArcAtomicRef<T>,
		func: impl FnOnce(&T) -> Option<&U>,
	) -> Option<ArcAtomicRef<U>> {
		Some(ArcAtomicRef {
			arc: orig.arc,
			borrow: AtomicRef::filter_map(orig.borrow, func)?,
		})
	}

	pub fn clone(orig: &ArcAtomicRef<T>) -> ArcAtomicRef<T> {
		ArcAtomicRef {
			arc: orig.arc.clone(),
			borrow: AtomicRef::clone(&orig.borrow),
		}
	}
}

pub struct ArcAtomicRefMut<T: ?Sized + 'static> {
	#[allow(unused)]
	arc: Arc<dyn Any>,
	borrow: AtomicRefMut<'static, T>,
}

impl<T> ArcAtomicRefMut<T> {
	pub fn map<U: ?Sized>(
		orig: ArcAtomicRefMut<T>,
		func: impl FnOnce(&mut T) -> &mut U,
	) -> ArcAtomicRefMut<U> {
		ArcAtomicRefMut {
			arc: orig.arc,
			borrow: AtomicRefMut::map(orig.borrow, func),
		}
	}

	pub fn filter_map<U: ?Sized>(
		orig: ArcAtomicRefMut<T>,
		func: impl FnOnce(&mut T) -> Option<&mut U>,
	) -> Option<ArcAtomicRefMut<U>> {
		Some(ArcAtomicRefMut {
			arc: orig.arc,
			borrow: AtomicRefMut::filter_map(orig.borrow, func)?,
		})
	}
}

impl<T> Deref for ArcAtomicRefMut<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.borrow
	}
}

impl<T> DerefMut for ArcAtomicRefMut<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.borrow
	}
}

pub trait ArcAtomicRefCellExt<T: ?Sized> {
	fn into_borrow(self) -> ArcAtomicRef<T>;
	fn into_borrow_mut(self) -> ArcAtomicRefMut<T>;
}

impl<T> ArcAtomicRefCellExt<T> for Arc<AtomicRefCell<T>> {
	fn into_borrow(self) -> ArcAtomicRef<T> {
		let borrow =
			unsafe { std::mem::transmute::<AtomicRef<'_, T>, AtomicRef<'static, T>>(self.borrow()) };
		ArcAtomicRef { arc: self, borrow }
	}

	fn into_borrow_mut(self) -> ArcAtomicRefMut<T> {
		let borrow = unsafe {
			std::mem::transmute::<AtomicRefMut<'_, T>, AtomicRefMut<'static, T>>(self.borrow_mut())
		};
		ArcAtomicRefMut { arc: self, borrow }
	}
}
