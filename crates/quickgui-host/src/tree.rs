use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NodeTag {
    Root,
    View,
    Button,
    Text,
    Sentinel,
    Input,
    VirtualList,
    Svg,
    SwiftUiHost,
    SwiftUiButton,
    SwiftUiQuickGuiHost,
    SwiftUiPopover,
    SwiftUiPopoverTrigger,
    SwiftUiPopoverContent,
    Image,
    Shader,
    SwiftUiSlider,
    SwiftUiToggle,
    SwiftUiProgressView,
    SwiftUiStepper,
    SwiftUiTextField,
    SwiftUiPicker,
    SwiftUiDatePicker,
    SwiftUiColorPicker,
    SwiftUiGauge,
    Extension,
}

impl NodeTag {
    pub(super) fn decode(value: u8) -> std::result::Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::View),
            2 => Ok(Self::Button),
            3 => Ok(Self::Text),
            4 => Ok(Self::Sentinel),
            5 => Ok(Self::Input),
            7 => Ok(Self::VirtualList),
            9 => Ok(Self::Svg),
            10 => Ok(Self::SwiftUiHost),
            11 => Ok(Self::SwiftUiButton),
            12 => Ok(Self::SwiftUiQuickGuiHost),
            13 => Ok(Self::SwiftUiPopover),
            14 => Ok(Self::SwiftUiPopoverTrigger),
            15 => Ok(Self::SwiftUiPopoverContent),
            16 => Ok(Self::Image),
            17 => Ok(Self::Shader),
            18 => Ok(Self::SwiftUiSlider),
            19 => Ok(Self::SwiftUiToggle),
            20 => Ok(Self::SwiftUiProgressView),
            21 => Ok(Self::SwiftUiStepper),
            22 => Ok(Self::SwiftUiTextField),
            23 => Ok(Self::SwiftUiPicker),
            24 => Ok(Self::SwiftUiDatePicker),
            25 => Ok(Self::SwiftUiColorPicker),
            26 => Ok(Self::SwiftUiGauge),
            30 => Ok(Self::Extension),
            _ => Err(ProtocolError::new(format!("unknown node tag {value}"))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum PropertyValue {
    Bool(bool),
    Number(f32),
    Color(u32),
    String(Arc<str>),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct NativeNode {
    pub(super) tag: NodeTag,
    pub(super) parent: Option<u32>,
    pub(super) children: Vec<u32>,
    pub(super) text: Arc<str>,
    pub(super) properties: Vec<(u16, PropertyValue)>,
}

impl NativeNode {
    pub(super) fn new(tag: NodeTag) -> Self {
        Self {
            tag,
            parent: None,
            children: Vec::new(),
            text: Arc::from(""),
            properties: Vec::new(),
        }
    }

    pub(super) fn property(&self, key: u16) -> Option<&PropertyValue> {
        self.properties
            .binary_search_by_key(&key, |(property, _)| *property)
            .ok()
            .map(|index| &self.properties[index].1)
    }

    pub(super) fn number(&self, key: u16) -> Option<f32> {
        match self.property(key) {
            Some(PropertyValue::Number(value)) if value.is_finite() => Some(*value),
            _ => None,
        }
    }

    pub(super) fn boolean(&self, key: u16) -> Option<bool> {
        match self.property(key) {
            Some(PropertyValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub(super) fn string(&self, key: u16) -> Option<&str> {
        match self.property(key) {
            Some(PropertyValue::String(value)) => Some(value),
            _ => None,
        }
    }

    pub(super) fn color(&self, key: u16) -> Option<Color> {
        match self.property(key) {
            Some(PropertyValue::Color(value)) => Some(unpack_color(*value)),
            _ => None,
        }
    }

    pub(super) fn set_property(&mut self, key: u16, value: Option<PropertyValue>) {
        match self
            .properties
            .binary_search_by_key(&key, |(property, _)| *property)
        {
            Ok(index) => match value {
                Some(value) => self.properties[index].1 = value,
                None => {
                    self.properties.remove(index);
                }
            },
            Err(index) => {
                if let Some(value) = value {
                    self.properties.insert(index, (key, value));
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct NativeTree {
    pub(super) nodes: HashMap<u32, NativeNode>,
    pub(super) revision: u32,
}

impl Default for NativeTree {
    fn default() -> Self {
        let mut nodes = HashMap::with_capacity(64);
        nodes.insert(ROOT_NODE, NativeNode::new(NodeTag::Root));
        Self { nodes, revision: 0 }
    }
}

#[derive(Clone, Debug)]
pub(super) enum Mutation {
    Create {
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    },
    SetProperty {
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    },
    ReplaceText {
        id: u32,
        text: Arc<str>,
    },
    Insert {
        parent: u32,
        child: u32,
        before: Option<u32>,
    },
    Remove {
        parent: u32,
        child: u32,
    },
    Cleanup {
        parent: u32,
        children: Vec<u32>,
    },
}

pub(super) struct TreeTransaction<'a> {
    pub(super) base: &'a NativeTree,
    pub(super) overlay: HashMap<u32, Option<NativeNode>>,
}

impl<'a> TreeTransaction<'a> {
    pub(super) fn new(base: &'a NativeTree) -> Self {
        Self {
            base,
            overlay: HashMap::new(),
        }
    }

    pub(super) fn node(&self, id: u32) -> Option<&NativeNode> {
        match self.overlay.get(&id) {
            Some(node) => node.as_ref(),
            None => self.base.nodes.get(&id),
        }
    }

    pub(super) fn edit(&mut self, id: u32) -> std::result::Result<&mut NativeNode, ProtocolError> {
        if !self.overlay.contains_key(&id) {
            let node = self
                .base
                .nodes
                .get(&id)
                .cloned()
                .ok_or_else(|| ProtocolError::new(format!("node {id} does not exist")))?;
            self.overlay.insert(id, Some(node));
        }
        self.overlay
            .get_mut(&id)
            .and_then(Option::as_mut)
            .ok_or_else(|| ProtocolError::new(format!("node {id} was already removed")))
    }

    pub(super) fn create(
        &mut self,
        id: u32,
        tag: NodeTag,
        text: Arc<str>,
    ) -> std::result::Result<(), ProtocolError> {
        if id == ROOT_NODE {
            return Err(ProtocolError::new("node id 0 is reserved for the root"));
        }
        if self.node(id).is_some() {
            return Err(ProtocolError::new(format!("node {id} already exists")));
        }
        let mut node = NativeNode::new(tag);
        node.text = text;
        self.overlay.insert(id, Some(node));
        Ok(())
    }

    pub(super) fn set_property(
        &mut self,
        id: u32,
        key: u16,
        value: Option<PropertyValue>,
    ) -> std::result::Result<(), ProtocolError> {
        if !(1..=property::LAST).contains(&key) {
            return Err(ProtocolError::new(format!("unknown property code {key}")));
        }
        self.edit(id)?.set_property(key, value);
        Ok(())
    }

    pub(super) fn replace_text(
        &mut self,
        id: u32,
        text: Arc<str>,
    ) -> std::result::Result<(), ProtocolError> {
        let node = self.edit(id)?;
        if node.tag != NodeTag::Text {
            return Err(ProtocolError::new(format!("node {id} is not a text node")));
        }
        node.text = text;
        Ok(())
    }

    pub(super) fn insert(
        &mut self,
        parent: u32,
        child: u32,
        before: Option<u32>,
    ) -> std::result::Result<(), ProtocolError> {
        if parent == child {
            return Err(ProtocolError::new("a node cannot contain itself"));
        }
        if self.node(parent).is_none() || self.node(child).is_none() {
            return Err(ProtocolError::new(format!(
                "cannot insert missing node {child} into {parent}"
            )));
        }
        if before == Some(child) && self.node(child).and_then(|node| node.parent) == Some(parent) {
            return Ok(());
        }
        if let Some(anchor) = before
            && self.node(anchor).and_then(|node| node.parent) != Some(parent)
        {
            return Err(ProtocolError::new(format!(
                "anchor {anchor} is not a child of {parent}"
            )));
        }

        let mut ancestor = Some(parent);
        for _ in 0..MAX_TREE_DEPTH {
            let Some(id) = ancestor else {
                break;
            };
            if id == child {
                return Err(ProtocolError::new("insertion would create a cycle"));
            }
            ancestor = self.node(id).and_then(|node| node.parent);
        }
        if ancestor.is_some() {
            return Err(ProtocolError::new(format!(
                "tree depth exceeds {MAX_TREE_DEPTH}"
            )));
        }

        if let Some(previous_parent) = self.node(child).and_then(|node| node.parent) {
            self.edit(previous_parent)?
                .children
                .retain(|candidate| *candidate != child);
        }
        let index = before
            .and_then(|anchor| {
                self.node(parent)?
                    .children
                    .iter()
                    .position(|candidate| *candidate == anchor)
            })
            .unwrap_or_else(|| self.node(parent).map_or(0, |node| node.children.len()));
        self.edit(parent)?.children.insert(index, child);
        self.edit(child)?.parent = Some(parent);
        Ok(())
    }

    pub(super) fn remove(
        &mut self,
        parent: u32,
        child: u32,
    ) -> std::result::Result<(), ProtocolError> {
        if child == ROOT_NODE {
            return Err(ProtocolError::new("the root node cannot be removed"));
        }
        if self.node(child).and_then(|node| node.parent) != Some(parent) {
            return Err(ProtocolError::new(format!(
                "node {child} is not a child of {parent}"
            )));
        }
        self.edit(parent)?
            .children
            .retain(|candidate| *candidate != child);

        let mut pending = vec![child];
        let mut removed = 0_usize;
        while let Some(id) = pending.pop() {
            removed += 1;
            if removed > MAX_NODES {
                return Err(ProtocolError::new("removed subtree exceeds the node limit"));
            }
            let children = self
                .node(id)
                .map(|node| node.children.clone())
                .unwrap_or_default();
            pending.extend(children);
            self.overlay.insert(id, None);
        }
        Ok(())
    }

    pub(super) fn finish(
        self,
    ) -> std::result::Result<HashMap<u32, Option<NativeNode>>, ProtocolError> {
        let removed = self.overlay.values().filter(|node| node.is_none()).count();
        let inserted = self
            .overlay
            .iter()
            .filter(|(id, node)| node.is_some() && !self.base.nodes.contains_key(id))
            .count();
        let final_len = self
            .base
            .nodes
            .len()
            .saturating_sub(removed)
            .saturating_add(inserted);
        if final_len > MAX_NODES + 1 {
            return Err(ProtocolError::new(format!(
                "tree cannot contain more than {MAX_NODES} application nodes"
            )));
        }
        Ok(self.overlay)
    }
}

pub(super) fn commit_overlay(
    tree: &mut NativeTree,
    overlay: HashMap<u32, Option<NativeNode>>,
) -> u32 {
    let mut changed = false;
    for (id, node) in overlay {
        if tree.nodes.get(&id) == node.as_ref() {
            continue;
        }
        changed = true;
        match node {
            Some(node) => {
                tree.nodes.insert(id, node);
            }
            None => {
                tree.nodes.remove(&id);
            }
        }
    }
    if changed {
        tree.revision = tree.revision.wrapping_add(1).max(1);
    }
    tree.revision
}

pub(super) fn apply_mutations(
    tree: &mut NativeTree,
    mutations: Vec<Mutation>,
) -> std::result::Result<u32, ProtocolError> {
    let mut transaction = TreeTransaction::new(tree);
    for mutation in mutations {
        match mutation {
            Mutation::Create { id, tag, text } => transaction.create(id, tag, text)?,
            Mutation::SetProperty { id, key, value } => transaction.set_property(id, key, value)?,
            Mutation::ReplaceText { id, text } => transaction.replace_text(id, text)?,
            Mutation::Insert {
                parent,
                child,
                before,
            } => transaction.insert(parent, child, before)?,
            Mutation::Remove { parent, child } => transaction.remove(parent, child)?,
            Mutation::Cleanup { parent, children } => {
                for child in children {
                    if transaction.node(child).is_some() {
                        transaction.remove(parent, child)?;
                    }
                }
            }
        }
    }
    let overlay = transaction.finish()?;
    Ok(commit_overlay(tree, overlay))
}

/// Translate ordinary signal writes into the core's targeted retained updates. Structural,
/// listener, layout, and component-owned changes use the normal declaration path. The core is
/// still responsible for layout dirtiness, inheritance, selection, and accessibility.
pub(super) fn retained_element_updates(
    tree: &NativeTree,
    mutations: &[Mutation],
) -> Option<Vec<quickgui::ElementUpdate>> {
    use quickgui::ElementUpdate;
    let mut updates = Vec::with_capacity(mutations.len());
    for mutation in mutations {
        let (id, update) = match mutation {
            Mutation::ReplaceText { id, text } => (
                *id,
                ElementUpdate::Text {
                    id: ElementId::new(*id as u64),
                    content: text.clone(),
                },
            ),
            Mutation::SetProperty {
                id,
                key,
                value: Some(PropertyValue::Color(color)),
            } => {
                let element_id = ElementId::new(*id as u64);
                let color = unpack_color(*color);
                let update = match *key {
                    property::BACKGROUND_COLOR => ElementUpdate::BackgroundColor {
                        id: element_id,
                        color,
                    },
                    property::COLOR => ElementUpdate::TextColor {
                        id: element_id,
                        color,
                    },
                    _ => return None,
                };
                (*id, update)
            }
            Mutation::SetProperty {
                id,
                key: property::OPACITY,
                value: Some(PropertyValue::Number(opacity)),
            } if opacity.is_finite() => (
                *id,
                ElementUpdate::Opacity {
                    id: ElementId::new(*id as u64),
                    opacity: *opacity,
                },
            ),
            _ => return None,
        };
        if id == ROOT_NODE {
            return None;
        }
        if matches!(update, ElementUpdate::BackgroundColor { .. })
            && tree
                .nodes
                .get(&id)?
                .property(property::BACKGROUND_GRADIENT)
                .is_some()
        {
            // A gradient declaration is applied after the solid background, and may resolve
            // to a solid color itself. Preserve that precedence through a normal rebuild.
            return None;
        }
        let mut current = id;
        let mut mounted = false;
        for _ in 0..MAX_TREE_DEPTH {
            if current == ROOT_NODE {
                mounted = true;
                break;
            }
            let node = tree.nodes.get(&current)?;
            if node.string(property::PART).is_some()
                || !matches!(node.tag, NodeTag::View | NodeTag::Button | NodeTag::Text)
            {
                return None;
            }
            current = node.parent?;
        }
        if !mounted {
            return None;
        }
        updates.push(update);
    }
    Some(updates)
}

/// An ordinary node keeps its native identity. Compound parts and collection-owned descendants
/// have derived IDs and coordinated callbacks, so their enclosing declaration remains the owner.
pub(super) fn native_scope_identity(tree: &NativeTree, id: u32) -> bool {
    let mut current = id;
    for _ in 0..MAX_TREE_DEPTH {
        if current == ROOT_NODE {
            return id != ROOT_NODE;
        }
        let Some(node) = tree.nodes.get(&current) else {
            return false;
        };
        if node.string(property::PART).is_some() {
            return false;
        }
        if current != id && !matches!(node.tag, NodeTag::View | NodeTag::Button) {
            return false;
        }
        if !matches!(
            node.tag,
            NodeTag::View
                | NodeTag::Button
                | NodeTag::Text
                | NodeTag::Sentinel
                | NodeTag::Input
                | NodeTag::VirtualList
                | NodeTag::Svg
                | NodeTag::Image
                | NodeTag::Shader
                | NodeTag::Extension
        ) {
            return false;
        }
        let Some(parent) = node.parent else {
            return false;
        };
        current = parent;
    }
    false
}

pub(super) fn native_scope_supported(tree: &NativeTree, id: u32) -> bool {
    if !native_scope_identity(tree, id) {
        return false;
    }
    let mut pending = vec![id];
    while let Some(id) = pending.pop() {
        let Some(node) = tree.nodes.get(&id) else {
            return false;
        };
        // Parts may hoist portals and coordinate siblings outside this subtree. Keep their
        // established full-declaration path until they expose an explicit component boundary.
        if node.string(property::PART).is_some() {
            return false;
        }
        pending.extend(&node.children);
    }
    true
}

/// Collect old mounted owners before committing the transaction. Newly constructed nodes are
/// covered by the insertion's existing parent. A keyed move invalidates both old and new parents.
pub(super) fn retained_scope_updates(
    tree: &NativeTree,
    mutations: &[Mutation],
) -> Option<Vec<ElementId>> {
    let mut targets = HashSet::new();
    let mut add = |id: u32| -> Option<()> {
        let Some(_) = tree.nodes.get(&id) else {
            return Some(());
        };
        let mut current = id;
        loop {
            if current == ROOT_NODE {
                return None;
            }
            let node = tree.nodes.get(&current)?;
            if native_scope_identity(tree, current) {
                if !native_scope_supported(tree, current) {
                    return None;
                }
                targets.insert(ElementId::new(u64::from(current)));
                return Some(());
            }
            let Some(parent) = node.parent else {
                return Some(());
            };
            current = parent;
        }
    };
    for mutation in mutations {
        match mutation {
            Mutation::Create { .. } => {}
            Mutation::SetProperty { id, .. } | Mutation::ReplaceText { id, .. } => add(*id)?,
            Mutation::Insert { parent, child, .. } => {
                add(*parent)?;
                if let Some(parent) = tree.nodes.get(child).and_then(|node| node.parent) {
                    add(parent)?;
                }
            }
            Mutation::Remove { parent, .. } | Mutation::Cleanup { parent, .. } => add(*parent)?,
        }
    }
    Some(targets.into_iter().collect())
}

#[derive(Debug)]
pub(super) struct ProtocolError(String);

impl ProtocolError {
    pub(super) fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) offset: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(super) fn take(&mut self, length: usize) -> std::result::Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| ProtocolError::new("truncated mutation batch"))?;
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    pub(super) fn u8(&mut self) -> std::result::Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    pub(super) fn u16(&mut self) -> std::result::Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    pub(super) fn u32(&mut self) -> std::result::Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub(super) fn f32(&mut self) -> std::result::Result<f32, ProtocolError> {
        let value = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ProtocolError::new("property numbers must be finite"))
        }
    }

    pub(super) fn string(&mut self) -> std::result::Result<Arc<str>, ProtocolError> {
        self.bounded_string(MAX_STRING_BYTES)
    }

    fn bounded_string(&mut self, limit: usize) -> std::result::Result<Arc<str>, ProtocolError> {
        let length = self.u32()? as usize;
        if length > limit {
            return Err(ProtocolError::new(format!(
                "strings cannot exceed {limit} bytes"
            )));
        }
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| ProtocolError::new("strings must contain valid UTF-8"))?;
        Ok(Arc::from(value))
    }

    pub(super) fn finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

pub(super) fn decode_batch(bytes: &[u8]) -> std::result::Result<Vec<Mutation>, ProtocolError> {
    if bytes.len() > MAX_BATCH_BYTES {
        return Err(ProtocolError::new(format!(
            "one mutation batch cannot exceed {MAX_BATCH_BYTES} bytes"
        )));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != PROTOCOL_MAGIC {
        return Err(ProtocolError::new("invalid mutation batch magic"));
    }
    let version = reader.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::new(format!(
            "unsupported mutation protocol version {version}"
        )));
    }
    let count = reader.u32()? as usize;
    if count > MAX_MUTATIONS {
        return Err(ProtocolError::new(format!(
            "one batch cannot contain more than {MAX_MUTATIONS} mutations"
        )));
    }
    let mut mutations = Vec::with_capacity(count.min(4_096));
    for _ in 0..count {
        let mutation = match reader.u8()? {
            1 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::decode(reader.u8()?)?,
                text: Arc::from(""),
            },
            2 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Text,
                text: reader.string()?,
            },
            3 => Mutation::Create {
                id: reader.u32()?,
                tag: NodeTag::Sentinel,
                text: Arc::from(""),
            },
            4 => {
                let id = reader.u32()?;
                let key = reader.u16()?;
                let value = match reader.u8()? {
                    0 => None,
                    1 => Some(PropertyValue::Bool(match reader.u8()? {
                        0 => false,
                        1 => true,
                        value => {
                            return Err(ProtocolError::new(format!(
                                "invalid boolean byte {value}"
                            )));
                        }
                    })),
                    2 => Some(PropertyValue::Number(reader.f32()?)),
                    3 => Some(PropertyValue::Color(reader.u32()?)),
                    4 => Some(PropertyValue::String(reader.bounded_string(
                        if key == property::EXTENSION_PROPS {
                            MAX_EXTENSION_PROPS_BYTES
                        } else {
                            MAX_STRING_BYTES
                        },
                    )?)),
                    value => {
                        return Err(ProtocolError::new(format!(
                            "unknown property value tag {value}"
                        )));
                    }
                };
                Mutation::SetProperty { id, key, value }
            }
            5 => Mutation::ReplaceText {
                id: reader.u32()?,
                text: reader.string()?,
            },
            6 => {
                let parent = reader.u32()?;
                let child = reader.u32()?;
                let anchor = reader.u32()?;
                Mutation::Insert {
                    parent,
                    child,
                    before: (anchor != NO_ANCHOR).then_some(anchor),
                }
            }
            7 => Mutation::Remove {
                parent: reader.u32()?,
                child: reader.u32()?,
            },
            8 => {
                let parent = reader.u32()?;
                let count = reader.u32()? as usize;
                if count > MAX_MUTATIONS {
                    return Err(ProtocolError::new(
                        "cleanup list exceeds the mutation limit",
                    ));
                }
                let mut children = Vec::with_capacity(count.min(4_096));
                for _ in 0..count {
                    children.push(reader.u32()?);
                }
                Mutation::Cleanup { parent, children }
            }
            opcode => {
                return Err(ProtocolError::new(format!(
                    "unknown mutation opcode {opcode}"
                )));
            }
        };
        mutations.push(mutation);
    }
    if !reader.finished() {
        return Err(ProtocolError::new("mutation batch has trailing bytes"));
    }
    Ok(mutations)
}

#[derive(Clone, Debug)]
pub(super) struct QueuedEvent {
    pub(super) kind: &'static str,
    pub(super) window: u32,
    pub(super) target: u32,
    pub(super) value: Option<Arc<str>>,
}

pub(super) type EventQueue = Rc<RefCell<VecDeque<QueuedEvent>>>;

const MAX_LIST_ITEM_HEIGHTS_JSON_BYTES: usize = MAX_STRING_BYTES;

fn list_item_heights_source(node: &NativeNode) -> Option<Arc<str>> {
    match node.property(property::ITEM_HEIGHTS) {
        Some(PropertyValue::String(source)) => Some(Arc::clone(source)),
        _ => None,
    }
}

pub(super) fn decode_list_item_heights(
    source: Option<&Arc<str>>,
    item_count: usize,
) -> Option<Vec<f32>> {
    let source = source?.as_ref();
    if source.len() > MAX_LIST_ITEM_HEIGHTS_JSON_BYTES {
        return None;
    }
    let heights = serde_json::from_str::<Vec<f64>>(source).ok()?;
    (heights.len() == item_count
        && heights
            .iter()
            .all(|height| height.is_finite() && *height > 0.0))
    .then(|| {
        heights
            .into_iter()
            .map(|height| height.min(f64::from(quickgui::MAX_LIST_ITEM_HEIGHT)) as f32)
            .collect()
    })
}

fn same_list_item_heights_source(left: Option<&Arc<str>>, right: Option<&Arc<str>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Arc::ptr_eq(left, right) || left == right,
        (None, None) => true,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativeListConfig {
    pub(super) estimated_item_height: f32,
    pub(super) overscan: usize,
    pub(super) overscan_pixels: f32,
    pub(super) alignment: ListAlignment,
    pub(super) follow_mode: FollowMode,
}

impl NativeListConfig {
    pub(super) fn from_node(node: &NativeNode) -> Self {
        let estimated_item_height = node
            .number(property::ESTIMATED_ITEM_HEIGHT)
            .unwrap_or(160.0)
            .clamp(1.0, quickgui::MAX_LIST_ITEM_HEIGHT);
        let overscan = node.number(property::OVERSCAN).unwrap_or(2.0).max(0.0) as usize;
        let overscan_pixels = node
            .number(property::OVERSCAN_PIXELS)
            .unwrap_or(0.0)
            .clamp(0.0, quickgui::MAX_LIST_ITEM_HEIGHT);
        let alignment = match node.string(property::LIST_ALIGNMENT) {
            Some("bottom") => ListAlignment::Bottom,
            _ => ListAlignment::Top,
        };
        let follow_mode = match node.string(property::FOLLOW_MODE) {
            Some("tail") => FollowMode::Tail,
            _ => FollowMode::Normal,
        };
        Self {
            estimated_item_height,
            overscan,
            overscan_pixels,
            alignment,
            follow_mode,
        }
    }

    pub(super) fn create_state(self, item_count: usize) -> ListState {
        ListState::new(item_count, self.estimated_item_height)
            .with_overscan(self.overscan)
            .with_overscan_pixels(self.overscan_pixels)
            .with_alignment(self.alignment)
            .with_follow_mode(self.follow_mode)
    }
}

pub(super) struct NativeListState {
    pub(super) config: NativeListConfig,
    pub(super) children: Vec<u32>,
    pub(super) list: ListState,
    item_heights_source: Option<Arc<str>>,
    has_item_heights: bool,
}

pub(super) struct NativeSvgState {
    pub(super) source: Arc<str>,
    pub(super) parsed: std::result::Result<Svg, Arc<str>>,
}

impl NativeSvgState {
    pub(super) fn new(source: Arc<str>) -> Self {
        let parsed = Svg::from_svg(source.as_ref()).map_err(|error| Arc::from(error.to_string()));
        Self { source, parsed }
    }

    pub(super) fn sync(&mut self, source: &str) {
        if self.source.as_ref() == source {
            return;
        }
        *self = Self::new(Arc::from(source));
    }

    pub(super) fn element(&self) -> Element {
        match &self.parsed {
            Ok(svg) => svg_element(svg),
            Err(_) => div().hidden(),
        }
    }
}

impl NativeListState {
    pub(super) fn new(node: &NativeNode) -> Self {
        let config = NativeListConfig::from_node(node);
        let item_heights_source = list_item_heights_source(node);
        let item_heights =
            decode_list_item_heights(item_heights_source.as_ref(), node.children.len());
        let list = config.create_state(node.children.len());
        if let Some(heights) = &item_heights {
            list.set_item_heights(heights);
        }
        Self {
            config,
            children: node.children.clone(),
            list,
            item_heights_source,
            has_item_heights: item_heights.is_some(),
        }
    }

    pub(super) fn sync(&mut self, node: &NativeNode) {
        let config = NativeListConfig::from_node(node);
        let item_heights_source = list_item_heights_source(node);
        let item_heights_changed = !same_list_item_heights_source(
            self.item_heights_source.as_ref(),
            item_heights_source.as_ref(),
        );
        if self.config != config {
            self.config = config;
            self.children.clone_from(&node.children);
            self.list = config.create_state(self.children.len());
            let item_heights =
                decode_list_item_heights(item_heights_source.as_ref(), self.children.len());
            if let Some(heights) = &item_heights {
                self.list.set_item_heights(heights);
            }
            self.item_heights_source = item_heights_source;
            self.has_item_heights = item_heights.is_some();
            return;
        }
        let children_changed = self.children != node.children;
        if !children_changed && !item_heights_changed {
            return;
        }
        let mut measurements_reset = false;
        if children_changed {
            let stable_prefix = self.children.starts_with(&node.children)
                || node.children.starts_with(&self.children);
            if stable_prefix {
                self.list.set_item_count(node.children.len());
            } else {
                self.list.reset(node.children.len());
                measurements_reset = true;
            }
            self.children.clone_from(&node.children);
        }
        let item_heights =
            decode_list_item_heights(item_heights_source.as_ref(), self.children.len());
        if let Some(heights) = &item_heights {
            self.list.set_item_heights(heights);
        } else if self.has_item_heights && !measurements_reset {
            self.list.remeasure();
        }
        self.item_heights_source = item_heights_source;
        self.has_item_heights = item_heights.is_some();
    }
}

/// Retained decoded image source for one declared `image` node.
///
/// A filesystem path stays a lazy core `ImageResource` so decoding runs on the core's bounded
/// worker pool; an inline `data:` URL is decoded once and retained until its declaration changes.
pub(super) struct NativeImageState {
    pub(super) source: Arc<str>,
    pub(super) parsed: std::result::Result<quickgui::ImageSource, Arc<str>>,
}

impl NativeImageState {
    pub(super) fn new(source: Arc<str>) -> Self {
        let parsed = native_image_source(source.as_ref());
        Self { source, parsed }
    }

    pub(super) fn sync(&mut self, source: &str) {
        if self.source.as_ref() == source {
            return;
        }
        *self = Self::new(Arc::from(source));
    }

    pub(super) fn element(&self, node: &NativeNode) -> Element {
        let Ok(source) = &self.parsed else {
            return div().hidden();
        };
        let mut element = quickgui::img(source.clone());
        if let Some(fit) = node.string(property::OBJECT_FIT) {
            element = element.object_fit(match fit {
                "fill" => quickgui::ObjectFit::Fill,
                "cover" => quickgui::ObjectFit::Cover,
                "scale-down" => quickgui::ObjectFit::ScaleDown,
                "none" => quickgui::ObjectFit::None,
                _ => quickgui::ObjectFit::Contain,
            });
        }
        element
    }
}

pub(super) fn native_image_source(
    source: &str,
) -> std::result::Result<quickgui::ImageSource, Arc<str>> {
    if source.is_empty() {
        return Err(Arc::from("an image source cannot be empty"));
    }
    if let Some(rest) = source.strip_prefix("data:") {
        let (header, payload) = rest
            .split_once(',')
            .ok_or_else(|| Arc::<str>::from("a data URL image needs a comma separator"))?;
        if !header.contains("base64") {
            return Err(Arc::from("only base64 data URL images are supported"));
        }
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload.trim())
                .map_err(|_| Arc::<str>::from("a data URL image is not valid base64"))?;
        // Animated formats keep their frames and repeat policy inside the core decoder.
        if let Ok(animated) = quickgui::AnimatedImage::decode(&bytes)
            && animated.frame_count() > 1
        {
            return Ok(quickgui::ImageSource::from(animated));
        }
        return quickgui::Image::decode(&bytes)
            .map(quickgui::ImageSource::from)
            .map_err(|error| Arc::from(error.to_string()));
    }
    let path = source.strip_prefix("file://").unwrap_or(source);
    Ok(quickgui::ImageSource::from(
        quickgui::ImageResource::from_path(path),
    ))
}

/// Retained validated WGSL for one declared `shader` node.
pub(super) struct NativeShaderState {
    pub(super) source: Arc<str>,
    pub(super) parsed: std::result::Result<quickgui::CustomShader, Arc<str>>,
}

impl NativeShaderState {
    pub(super) fn new(source: Arc<str>) -> Self {
        let parsed = quickgui::CustomShader::new(source.as_ref())
            .map_err(|error| Arc::from(error.to_string()));
        Self { source, parsed }
    }

    pub(super) fn sync(&mut self, source: &str) {
        if self.source.as_ref() == source {
            return;
        }
        *self = Self::new(Arc::from(source));
    }

    pub(super) fn element(&self, node: &NativeNode) -> Element {
        let Ok(shader) = &self.parsed else {
            return div().hidden();
        };
        quickgui::custom_shader(shader.clone()).shader_parameters(native_shader_parameters(node))
    }
}

/// Decode the declared bounded shader parameter vectors.
///
/// The core exposes exactly `CUSTOM_SHADER_PARAMETER_VECTORS` four-component vectors, so extra
/// declared values are ignored instead of growing the uniform.
pub(super) fn native_shader_parameters(node: &NativeNode) -> quickgui::ShaderParameters {
    let mut parameters = quickgui::ShaderParameters::new();
    let Some(declared) = node.string(property::SHADER_PARAMETERS) else {
        return parameters;
    };
    let Ok(values) = serde_json::from_str::<Vec<f32>>(declared) else {
        return parameters;
    };
    for (index, value) in values.into_iter().enumerate() {
        if index >= quickgui::CUSTOM_SHADER_PARAMETER_VECTORS * 4 {
            break;
        }
        parameters = parameters.float(index, value);
    }
    parameters
}
