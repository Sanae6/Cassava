use std::{
	any::{Any, TypeId},
	collections::BTreeMap,
	ops::DerefMut,
};

use gpui::{AnyEntity, AnyWeakEntity, App, BorrowAppContext, Context, Entity};

pub struct EventBus {
	handlers: BTreeMap<TypeId, Vec<(AnyWeakEntity, Box<dyn Fn(&dyn Any, &mut App)>)>>,
}

impl EventBus {
	pub fn register(cx: &mut App) {
		cx.set_global(Self {
			handlers: Default::default(),
		});
	}
	fn emit<Evt: 'static>(&mut self, cx: &mut App, event: Evt) {
		if let Some(handlers) = self.handlers.get_mut(&TypeId::of::<Evt>()) {
			handlers.retain(|(entity, handler)| {
				if entity.is_upgradable() {
					handler(&event, cx);
				}
				entity.is_upgradable()
			});
		}
	}

	fn push_handler<Evt: 'static>(
		&mut self,
		entity: AnyEntity,
		func: impl Fn(&Evt, &mut App) + 'static,
	) {
		self
			.handlers
			.entry(TypeId::of::<Evt>())
			.or_insert_with(Vec::new)
			.push((
				entity.downgrade(),
				Box::new(move |event, cx| func(event.downcast_ref().unwrap(), cx)),
			))
	}
}

impl gpui::Global for EventBus {}

pub trait AppEventExt {
	fn bus_subscribe<Evt: 'static, E: 'static>(
		&mut self,
		entity: Entity<E>,
		func: impl Fn(&Evt, &mut App) + 'static,
	);

	fn bus_emit<Evt: 'static>(&mut self, event: Evt);
}
impl AppEventExt for App {
	fn bus_subscribe<Evt: 'static, E: 'static>(
		&mut self,
		entity: Entity<E>,
		func: impl Fn(&Evt, &mut App) + 'static,
	) {
		self.update_global(|bus: &mut EventBus, _| {
			bus.push_handler(entity.into_any(), func);
		})
	}

	fn bus_emit<Evt: 'static>(&mut self, event: Evt) {
		self.update_global(|bus: &mut EventBus, cx| {
			bus.emit(cx, event);
		})
	}
}
pub trait ContextExt<E> {
	fn bus_subscribe<Evt: 'static>(&mut self, func: impl Fn(&mut E, &Evt, &mut App) + 'static);
}

impl<E: 'static> ContextExt<E> for Context<'_, E> {
	fn bus_subscribe<Evt: 'static>(&mut self, func: impl Fn(&mut E, &Evt, &mut App) + 'static) {
		let entity = self.entity();
		AppEventExt::bus_subscribe(self.deref_mut(), entity.clone(), move |event, cx| {
			entity.update(cx, |entity, cx| func(entity, event, cx))
		});
	}
}
