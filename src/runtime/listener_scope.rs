use super::*;

/// Callback keys survive updates to unrelated components. Retired keys are never reused, so a
/// queued hover exit or action cannot accidentally call a new component's listener.
pub(super) struct ListenerSlots<C> {
    values: HashMap<u64, C>,
    next: u64,
}

impl<C> Default for ListenerSlots<C> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            next: 0,
        }
    }
}

impl<C> ListenerSlots<C> {
    pub(super) fn len(&self) -> usize {
        self.values.len()
    }
    pub(super) fn push(&mut self, callback: C) -> u64 {
        let key = self.next;
        self.next = self
            .next
            .checked_add(1)
            .expect("listener key space exhausted");
        self.values.insert(key, callback);
        key
    }
    pub(super) fn get(&self, key: u64) -> Option<&C> {
        self.values.get(&key)
    }
    pub(super) fn remove(&mut self, key: u64) {
        self.values.remove(&key);
    }
    pub(super) fn clear(&mut self) {
        self.values.clear();
    }
}

pub(super) enum OwnedListener {
    Click(ElementId),
    Pointer(ElementId),
    ContextMenu(ElementId),
    ScrollWheel(ElementId),
    Touch(ElementId),
    Pressure(ElementId),
    Pinch(ElementId),
    Rotation(ElementId),
    SmartMagnify(ElementId),
    Drag(ElementId),
    Drop(ElementId, TypeId),
    Input(ElementId),
    Submit(ElementId),
    FormSubmit(ElementId),
    FormInvalid(ElementId),
    Dismiss(ElementId),
    Mouse(u64),
    Key(u64),
    Action(u64),
    ChildClosed(WindowHandle),
    AnyChildClosed(ChildWindowClosedCallback),
    Subscription(Rc<SubscriptionState>),
}

impl ListenerRegistry {
    pub(super) fn own_listener(&mut self, listener: OwnedListener) {
        if let Some(owner) = self.current_scope {
            self.scope_listeners
                .entry(owner)
                .or_default()
                .push(listener);
        }
    }

    pub(super) fn clear_scope_listeners(&mut self, owner: ElementId) {
        for listener in self.scope_listeners.remove(&owner).unwrap_or_default() {
            match listener {
                OwnedListener::Click(id) => {
                    self.clicks.remove(&id);
                }
                OwnedListener::Pointer(id) => {
                    self.pointers.remove(&id);
                }
                OwnedListener::ContextMenu(id) => {
                    self.context_menus.remove(&id);
                }
                OwnedListener::ScrollWheel(id) => {
                    self.scroll_wheels.remove(&id);
                }
                OwnedListener::Touch(id) => {
                    self.touches.remove(&id);
                }
                OwnedListener::Pressure(id) => {
                    self.mouse_pressures.remove(&id);
                }
                OwnedListener::Pinch(id) => {
                    self.pinches.remove(&id);
                }
                OwnedListener::Rotation(id) => {
                    self.rotations.remove(&id);
                }
                OwnedListener::SmartMagnify(id) => {
                    self.smart_magnifies.remove(&id);
                }
                OwnedListener::Drag(id) => {
                    self.drag_sources.remove(&id);
                }
                OwnedListener::Drop(id, kind) => {
                    self.drops.remove(&(id, kind));
                    self.drop_order.retain(|key| *key != (id, kind));
                }
                OwnedListener::Input(id) => {
                    self.inputs.remove(&id);
                }
                OwnedListener::Submit(id) => {
                    self.submits.remove(&id);
                }
                OwnedListener::FormSubmit(id) => {
                    self.form_submits.remove(&id);
                }
                OwnedListener::FormInvalid(id) => {
                    self.form_invalids.remove(&id);
                }
                OwnedListener::Dismiss(id) => {
                    self.dismisses.remove(&id);
                }
                OwnedListener::Mouse(key) => self.mouse_listeners.remove(key),
                OwnedListener::Key(key) => self.key_listeners.remove(key),
                OwnedListener::Action(key) => self.actions.remove(key),
                OwnedListener::ChildClosed(child) => {
                    self.child_window_closed.remove(&child);
                }
                OwnedListener::AnyChildClosed(callback) => {
                    self.any_child_window_closed
                        .retain(|existing| !Arc::ptr_eq(existing, &callback));
                }
                OwnedListener::Subscription(state) => state.cancel(),
            }
        }
        self.prune_entity_event_subscriptions();
        self.prune_global_subscriptions();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_callback_keys_never_alias_new_listeners() {
        let mut slots = ListenerSlots::default();
        let value = Rc::new(());
        let first = slots.push(value.clone());
        let sibling = slots.push(value.clone());
        slots.remove(first);
        assert_eq!(Rc::strong_count(&value), 2);
        for _ in 0..10_000 {
            let key = slots.push(value.clone());
            assert_ne!(key, first);
            assert!(slots.get(first).is_none());
            assert!(slots.get(sibling).is_some());
            slots.remove(key);
        }
        assert_eq!(slots.len(), 1);
        slots.clear();
        let next = slots.push(value);
        assert!(slots.get(sibling).is_none());
        assert_ne!(next, sibling);
    }

    #[test]
    fn disposing_one_scope_releases_its_callbacks_and_subscriptions_only() {
        let mut listeners = ListenerRegistry::default();
        let first = ElementId::new(1);
        let second = ElementId::new(2);
        let retained = Arc::new(());
        listeners.current_scope = Some(first);
        let capture = retained.clone();
        let key = listeners.push_mouse_listener(Arc::new(move |_, _, _| {
            let _ = &capture;
        }));
        let subscription = listeners.subscribe_entity_event(
            Entity::new(()).id(),
            TypeId::of::<()>(),
            Rc::new(RefCell::new(
                |_: &mut dyn Any, _: &dyn Any, _: &mut EventContext| {},
            )),
        );
        listeners.current_scope = Some(second);
        let sibling = listeners.push_mouse_listener(Arc::new(|_, _, _| {}));
        listeners.clear_scope_listeners(first);
        assert!(listeners.mouse_listener(key).is_none());
        assert!(listeners.mouse_listener(sibling).is_some());
        assert_eq!(Arc::strong_count(&retained), 1);
        assert_eq!(listeners.entity_subscription_count, 0);
        drop(subscription);
    }
}
