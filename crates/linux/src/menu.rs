use std::collections::HashMap;
use zbus::{interface, zvariant::Value};

const DBUS_MENU_PATH: &str = "/MenuBar";

pub struct DBusMenu {
    items: HashMap<i32, MenuItem>,
    next_id: i32,
}

struct MenuItem {
    id: i32,
    label: String,
    enabled: bool,
    visible: bool,
    item_type: MenuItemType,
    children: Vec<i32>,
}

enum MenuItemType {
    Standard,
    Separator,
}

impl DBusMenu {
    pub fn new() -> Self {
        let mut items = HashMap::new();
        items.insert(0, MenuItem {
            id: 0,
            label: String::new(),
            enabled: true,
            visible: true,
            item_type: MenuItemType::Standard,
            children: Vec::new(),
        });

        Self {
            items,
            next_id: 1,
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.items.insert(0, MenuItem {
            id: 0,
            label: String::new(),
            enabled: true,
            visible: true,
            item_type: MenuItemType::Standard,
            children: Vec::new(),
        });
        self.next_id = 1;
    }

    pub fn add_item(&mut self, label: impl Into<String>, parent_id: i32) -> i32 {
        let id = self.next_id;
        self.next_id += 1;

        let item = MenuItem {
            id,
            label: label.into(),
            enabled: true,
            visible: true,
            item_type: MenuItemType::Standard,
            children: Vec::new(),
        };

        self.items.insert(id, item);

        if let Some(parent) = self.items.get_mut(&parent_id) {
            parent.children.push(id);
        }

        id
    }

    pub fn add_separator(&mut self, parent_id: i32) -> i32 {
        let id = self.next_id;
        self.next_id += 1;

        let item = MenuItem {
            id,
            label: String::new(),
            enabled: false,
            visible: true,
            item_type: MenuItemType::Separator,
            children: Vec::new(),
        };

        self.items.insert(id, item);

        if let Some(parent) = self.items.get_mut(&parent_id) {
            parent.children.push(id);
        }

        id
    }

    fn item_to_properties(&self, item: &MenuItem, property_names: &[String]) -> Vec<(String, Value<'_>)> {
        let mut props = Vec::new();
        let include_all = property_names.is_empty();

        if include_all || property_names.iter().any(|p| p == "label") {
            props.push(("label".to_string(), Value::from(item.label.clone())));
        }

        if include_all || property_names.iter().any(|p| p == "enabled") {
            props.push(("enabled".to_string(), Value::from(item.enabled)));
        }

        if include_all || property_names.iter().any(|p| p == "visible") {
            props.push(("visible".to_string(), Value::from(item.visible)));
        }

        if include_all || property_names.iter().any(|p| p == "type") {
            let type_str = match item.item_type {
                MenuItemType::Standard => "standard",
                MenuItemType::Separator => "separator",
            };
            props.push(("type".to_string(), Value::from(type_str)));
        }

        if !item.children.is_empty() && (include_all || property_names.iter().any(|p| p == "children-display")) {
            props.push(("children-display".to_string(), Value::from("submenu")));
        }

        props
    }

    fn build_layout(&self, item_id: i32, recursion_depth: i32, property_names: &[String]) -> (i32, Vec<(String, Value<'_>)>, Vec<i32>) {
        let item = match self.items.get(&item_id) {
            Some(item) => item,
            None => return (item_id, Vec::new(), Vec::new()),
        };

        let properties = self.item_to_properties(item, property_names);

        let children = if recursion_depth != 0 {
            item.children.clone()
        } else {
            Vec::new()
        };

        (item.id, properties, children)
    }
}

#[interface(name = "com.canonical.dbusmenu")]
impl DBusMenu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }

    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> (u32, (i32, Vec<(String, Value<'_>)>, Vec<(i32, Vec<(String, Value<'_>)>, Vec<i32>)>)) {
        let (id, properties, children_ids) = self.build_layout(parent_id, recursion_depth, &property_names);

        let children: Vec<(i32, Vec<(String, Value<'_>)>, Vec<i32>)> = children_ids
            .iter()
            .map(|&child_id| self.build_layout(child_id, recursion_depth - 1, &property_names))
            .collect();

        (0, (id, properties, children))
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<String>,
    ) -> Vec<(i32, Vec<(String, Value<'_>)>)> {
        ids.into_iter()
            .filter_map(|id| {
                self.items.get(&id).map(|item| {
                    let props = self.item_to_properties(item, &property_names);
                    (id, props)
                })
            })
            .collect()
    }

    fn get_property(&self, id: i32, name: String) -> Value<'_> {
        self.items
            .get(&id)
            .and_then(|item| {
                self.item_to_properties(item, &[name.clone()])
                    .into_iter()
                    .find(|(k, _)| k == &name)
                    .map(|(_, v)| v)
            })
            .unwrap_or_else(|| Value::from(""))
    }

    fn event(&self, _id: i32, _event_id: String, _data: Value<'_>, _timestamp: u32) {}

    fn event_group(&self, _events: Vec<(i32, String, Value<'_>, u32)>) -> Vec<i32> {
        Vec::new()
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
}

pub fn menu_path() -> &'static str {
    DBUS_MENU_PATH
}
