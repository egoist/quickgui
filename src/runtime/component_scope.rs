use super::*;
use crate::ElementUpdate;

const MAX_COMPONENT_SCOPES: usize = 262_144;

type ScopeCallback = Rc<RefCell<dyn FnMut(&mut ViewContext<'_, ()>) -> ScopeOutput>>;

pub(super) enum ScopeOutput {
    Element(Box<Element>),
    Update(ElementUpdate),
}

pub(super) struct ComponentScope {
    parent: Option<ElementId>,
    callback: Option<ScopeCallback>,
    entities: HashSet<EntityId>,
    globals: HashSet<TypeId>,
    children: HashSet<ElementId>,
    order: u64,
}

pub(super) struct PendingComponent {
    pub id: ElementId,
    parent: Option<ElementId>,
    callback: Option<ScopeCallback>,
}

#[derive(Default)]
pub(super) struct ComponentScopes {
    entries: HashMap<ElementId, ComponentScope>,
    entities: HashMap<EntityId, HashSet<ElementId>>,
    globals: HashMap<TypeId, HashSet<ElementId>>,
    dirty: HashSet<ElementId>,
    next_order: u64,
}

impl ComponentScopes {
    pub(super) fn observe_entity(&mut self, scope: ElementId, entity: EntityId) {
        assert!(
            self.entities.contains_key(&entity)
                || self.entities.len() < MAX_OBSERVED_ENTITIES_PER_WINDOW,
            "too many scoped entity dependencies"
        );
        self.entries
            .get_mut(&scope)
            .expect("mounted component scope")
            .entities
            .insert(entity);
        self.entities.entry(entity).or_default().insert(scope);
    }

    pub(super) fn observe_global(&mut self, scope: ElementId, global: TypeId) {
        assert!(
            self.globals.contains_key(&global)
                || self.globals.len() < MAX_OBSERVED_GLOBALS_PER_WINDOW,
            "too many scoped global dependencies"
        );
        self.entries
            .get_mut(&scope)
            .expect("mounted component scope")
            .globals
            .insert(global);
        self.globals.entry(global).or_default().insert(scope);
    }

    pub(super) fn invalidate_entities(&mut self, entities: &[EntityId]) -> bool {
        let mut changed = false;
        for entity in entities {
            if let Some(scopes) = self.entities.get(entity) {
                changed |= !scopes.is_empty();
                self.dirty.extend(scopes);
            }
        }
        changed
    }

    pub(super) fn invalidate_globals(&mut self, globals: &[TypeId]) -> bool {
        let mut changed = false;
        for global in globals {
            if let Some(scopes) = self.globals.get(global) {
                changed |= !scopes.is_empty();
                self.dirty.extend(scopes);
            }
        }
        changed
    }

    pub(super) fn invalidate(&mut self, ids: &[ElementId]) -> bool {
        if !ids.iter().all(|id| self.entries.contains_key(id)) {
            return false;
        }
        self.dirty.extend(ids);
        true
    }

    pub(super) fn pending(&self) -> bool {
        !self.dirty.is_empty()
    }

    fn subtree(&self, owner: ElementId) -> Vec<ElementId> {
        let mut ids = vec![owner];
        let mut index = 0;
        while index < ids.len() {
            if let Some(scope) = self.entries.get(&ids[index]) {
                ids.extend(&scope.children);
            }
            index += 1;
        }
        ids
    }

    pub(super) fn take_dirty_roots(&mut self) -> Vec<ElementId> {
        let dirty = std::mem::take(&mut self.dirty);
        let mut roots: Vec<_> = dirty
            .iter()
            .copied()
            .filter(|id| {
                let mut parent = self.entries.get(id).and_then(|scope| scope.parent);
                while let Some(id) = parent {
                    if dirty.contains(&id) {
                        return false;
                    }
                    parent = self.entries.get(&id).and_then(|scope| scope.parent);
                }
                true
            })
            .collect();
        roots.sort_unstable_by_key(|id| self.entries.get(id).map(|scope| scope.order));
        roots
    }

    fn remove(&mut self, id: ElementId) {
        if let Some(scope) = self.entries.remove(&id) {
            if let Some(parent) = scope
                .parent
                .and_then(|parent| self.entries.get_mut(&parent))
            {
                parent.children.remove(&id);
            }
            for entity in scope.entities {
                if let Some(scopes) = self.entities.get_mut(&entity) {
                    scopes.remove(&id);
                    if scopes.is_empty() {
                        self.entities.remove(&entity);
                    }
                }
            }
            for global in scope.globals {
                if let Some(scopes) = self.globals.get_mut(&global) {
                    scopes.remove(&id);
                    if scopes.is_empty() {
                        self.globals.remove(&global);
                    }
                }
            }
        }
        self.dirty.remove(&id);
    }
}

impl ListenerRegistry {
    pub(super) fn needs_scoped_replacement(&self, updates: &[ElementUpdate]) -> bool {
        // An external replacement has no declaration context in which to register or retire
        // scoped callbacks. Re-enter the owning renderer instead of leaving detached subscribers
        // active or retaining callbacks that captured the previous component values.
        !self.scopes.entries.is_empty()
            && updates
                .iter()
                .any(|update| matches!(update, ElementUpdate::Replace { .. }))
    }

    pub(super) fn prepare_component_updates(&mut self) -> Vec<PendingComponent> {
        let pending: Vec<_> = self
            .scopes
            .take_dirty_roots()
            .into_iter()
            .filter_map(|id| {
                let scope = self.scopes.entries.get(&id)?;
                Some(PendingComponent {
                    id,
                    parent: scope.parent,
                    callback: scope.callback.clone(),
                })
            })
            .collect();
        // Retire all old roots before declaring any new root. A keyed child may move from a
        // later root to an earlier one in the same batch without a transient duplicate listener.
        for scope in &pending {
            self.clear_component(scope.id);
        }
        pending
    }

    fn remove_component(&mut self, id: ElementId) {
        self.clear_scope_listeners(id);
        self.scopes.remove(id);
    }

    fn clear_component(&mut self, owner: ElementId) {
        for id in self.scopes.subtree(owner).into_iter().rev() {
            self.remove_component(id);
        }
    }

    pub(super) fn retain_components(&mut self, root: &Element, owner: Option<ElementId>) {
        fn collect(element: &Element, ids: &mut HashSet<ElementId>) {
            if let Some(id) = element.explicit_id {
                ids.insert(id);
            }
            for child in &element.children {
                collect(child, ids);
            }
        }
        if self.scopes.entries.is_empty() {
            return;
        }
        let mut mounted = HashSet::new();
        collect(root, &mut mounted);
        let candidates = owner
            .map(|owner| self.scopes.subtree(owner))
            .unwrap_or_else(|| self.scopes.entries.keys().copied().collect());
        let removed: Vec<_> = candidates
            .into_iter()
            .filter(|id| !mounted.contains(id))
            .collect();
        for id in removed {
            self.remove_component(id);
        }
    }
}

impl<V: 'static> ViewContext<'_, V> {
    /// Declare a restartable component. Entity/global reads inside this callback belong to this
    /// component; notifying them re-executes only affected scopes. Capture retained `Entity`
    /// handles rather than a snapshot of their value. The returned root receives `id`: return an
    /// unkeyed root or one with that same ID, and put nested components in its children.
    pub fn component<E: IntoElement>(
        &mut self,
        id: impl Into<ElementId>,
        mut render: impl FnMut(&mut ViewContext<'_, V>) -> E + 'static,
    ) -> Element {
        let id = id.into();
        let callback: ScopeCallback = Rc::new(RefCell::new(move |cx: &mut ViewContext<'_, ()>| {
            ScopeOutput::Element(Box::new(
                cx.with_type::<V, _>(|cx| render(cx).into_element()),
            ))
        }));
        self.declare_component(id, Some(callback.clone()));
        let parent = self.listeners.current_scope.replace(id);
        let ScopeOutput::Element(element) = self.with_type::<(), _>(|cx| callback.borrow_mut()(cx))
        else {
            unreachable!()
        };
        self.listeners.current_scope = parent;
        element.id(id)
    }

    /// Attach a value binding to an explicitly identified element. The callback returns a direct
    /// text/color/opacity/transform update for that element. Its dependencies restart this binding
    /// without rebuilding the containing component; the update determines the affected phase.
    pub fn bind(
        &mut self,
        mut element: Element,
        mut read: impl FnMut(&mut ViewContext<'_, V>) -> ElementUpdate + 'static,
    ) -> Element {
        let id = element
            .explicit_id
            .expect("a bound element needs an explicit ID");
        let callback: ScopeCallback = Rc::new(RefCell::new(move |cx: &mut ViewContext<'_, ()>| {
            ScopeOutput::Update(cx.with_type::<V, _>(|cx| read(cx)))
        }));
        self.declare_component(id, Some(callback.clone()));
        let parent = self.listeners.current_scope.replace(id);
        let ScopeOutput::Update(update) = self.with_type::<(), _>(|cx| callback.borrow_mut()(cx))
        else {
            unreachable!()
        };
        self.listeners.current_scope = parent;
        assert!(
            update.apply_to_declaration(&mut element),
            "a binding must update its own element with a direct property"
        );
        self.assert_value_binding(id);
        element
    }

    /// Give an embedding renderer ownership of the listeners and dependencies declared inside
    /// this subtree. Implement `View::render_scope` to rebuild it after `invalidate_elements`.
    pub fn with_scope<E: IntoElement>(
        &mut self,
        id: impl Into<ElementId>,
        render: impl FnOnce(&mut Self) -> E,
    ) -> Element {
        let id = id.into();
        self.declare_component(id, None);
        let parent = self.listeners.current_scope.replace(id);
        let element = render(self).into_element().id(id);
        self.listeners.current_scope = parent;
        element
    }

    fn declare_component(&mut self, id: ElementId, callback: Option<ScopeCallback>) {
        let scopes = &mut self.listeners.scopes;
        assert!(
            scopes.entries.len() < MAX_COMPONENT_SCOPES,
            "too many component scopes in one window"
        );
        assert!(
            !scopes.entries.contains_key(&id),
            "component scope {id:?} was declared twice"
        );
        let scope = ComponentScope {
            parent: self.listeners.current_scope,
            callback,
            entities: HashSet::new(),
            globals: HashSet::new(),
            children: HashSet::new(),
            order: scopes.next_order,
        };
        if let Some(parent) = scope.parent {
            scopes
                .entries
                .get_mut(&parent)
                .expect("declaring component parent")
                .children
                .insert(id);
        }
        scopes.next_order = scopes
            .next_order
            .checked_add(1)
            .expect("component order exhausted");
        scopes.entries.insert(id, scope);
    }

    fn assert_value_binding(&self, id: ElementId) {
        assert!(
            self.listeners
                .scope_listeners
                .get(&id)
                .is_none_or(Vec::is_empty)
                && self.listeners.scopes.entries[&id].children.is_empty(),
            "value bindings cannot declare listeners or child components"
        );
    }

    pub(super) fn refresh_component(
        &mut self,
        pending: PendingComponent,
        render_external: impl FnOnce(&mut Self) -> Option<Element>,
    ) -> Option<ElementUpdate> {
        let PendingComponent {
            id,
            parent,
            callback,
        } = pending;
        self.listeners.current_scope = parent;
        let output = if let Some(callback) = callback {
            self.declare_component(id, Some(callback.clone()));
            self.listeners.current_scope = Some(id);
            self.with_type::<(), _>(|cx| callback.borrow_mut()(cx))
        } else {
            // The embedding renderer calls with_scope again while rebuilding this root.
            ScopeOutput::Element(Box::new(render_external(self)?))
        };
        self.listeners.current_scope = None;
        Some(match output {
            ScopeOutput::Element(element) => {
                let element = element.id(id);
                self.listeners.retain_components(&element, Some(id));
                ElementUpdate::Replace {
                    id,
                    element: Box::new(element),
                }
            }
            ScopeOutput::Update(update) => {
                assert_eq!(update.id(), id, "a binding cannot change another element");
                assert!(
                    !matches!(update, ElementUpdate::Replace { .. }),
                    "use a component for structural changes"
                );
                self.assert_value_binding(id);
                update
            }
        })
    }

    pub(super) fn with_type<T: 'static, R>(
        &mut self,
        call: impl FnOnce(&mut ViewContext<'_, T>) -> R,
    ) -> R {
        let mut cx = ViewContext::<T> {
            size: self.size,
            scale_factor: self.scale_factor,
            metrics: self.metrics,
            focused: self.focused,
            focused_path: self.focused_path.clone(),
            request_animation_frame: false,
            repaint_deadline: None,
            listeners: self.listeners,
            window: self.window,
            window_state: self.window_state,
            displays: self.displays,
            keyboard_layout: self.keyboard_layout,
            font_system: self.font_system,
            assets: self.assets,
            app_info: self.app_info,
            app_paths: self.app_paths,
            system_info: self.system_info,
            system_preferences: self.system_preferences,
            background_tasks: self.background_tasks,
            foreground_tasks: self.foreground_tasks,
            globals: self.globals,
            event_proxy: self.event_proxy,
            marker: PhantomData,
        };
        let result = call(&mut cx);
        self.request_animation_frame |= cx.request_animation_frame;
        self.repaint_deadline = self
            .repaint_deadline
            .into_iter()
            .chain(cx.repaint_deadline)
            .min();
        result
    }
}
