//! Typed access to a parsed save (spec §6.1): players, world objects and
//! the inventory linkage between them (port of the .NET `PlayerViewModel`).

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value};

use crate::format::{Object, SaveFile};

pub const TERRA_TOKENS: usize = 0;
pub const PLANETS: usize = 1;
pub const PLAYERS: usize = 2;
pub const WORLD_OBJECTS: usize = 3;
pub const INVENTORIES: usize = 4;
pub const STATS: usize = 5;
pub const DISPLAY_INFO: usize = 8;

/// An item shown in a player's inventory: its world object id and group id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub world_object_id: i64,
    pub gid: String,
}

/// A parsed save with an index of world objects by id.
#[derive(Debug, Clone)]
pub struct Save {
    file: SaveFile,
    /// World object id → position in the `WorldObjects` group (first wins).
    world_objects: HashMap<i64, usize>,
}

impl Save {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "the save is not valid UTF-8")?;
        let file = SaveFile::parse(text)?;
        let mut world_objects = HashMap::new();
        for (index, object) in file.group(WORLD_OBJECTS).objects().iter().enumerate() {
            if let Some(id) = int(object.get("id")) {
                world_objects.entry(id).or_insert(index);
            }
        }
        Ok(Self {
            file,
            world_objects,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.file.to_text().into_bytes()
    }

    pub fn single(&self, group: usize) -> &Object {
        self.file.single(group)
    }

    pub fn single_mut(&mut self, group: usize) -> &mut Object {
        self.file.single_mut(group)
    }

    /// One object per planet (Prime, Humble, …).
    pub fn planets(&self) -> &[Object] {
        self.file.group(PLANETS).objects()
    }

    pub fn planet_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.file.group_mut(PLANETS).objects_mut().get_mut(index)
    }

    pub fn players(&self) -> &[Object] {
        self.file.group(PLAYERS).objects()
    }

    pub fn player_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.file.group_mut(PLAYERS).objects_mut().get_mut(index)
    }

    /// The items of inventory `inventory_id`, in `woIds` order, skipping ids
    /// with no world object. `None` when the inventory does not exist.
    pub fn inventory_items(&self, inventory_id: i64) -> Option<Vec<Item>> {
        let inventory = self.inventory(inventory_id)?;
        let world = self.file.group(WORLD_OBJECTS).objects();
        let items = wo_ids(inventory)
            .into_iter()
            .filter_map(|id| {
                let object = &world[*self.world_objects.get(&id)?];
                Some(Item {
                    world_object_id: id,
                    gid: text(object.get("gId")).to_owned(),
                })
            })
            .collect();
        Some(items)
    }

    /// Creates world object `{ id: max + 1, gId }` and appends it to the
    /// inventory. Returns the new id, or `None` when the inventory does not
    /// exist.
    pub fn add_item(&mut self, inventory_id: i64, gid: &str) -> Option<i64> {
        let index = self.inventory_index(inventory_id)?;
        let id = self.world_objects.keys().max().map_or(1, |max| max + 1);
        let mut fields = Map::new();
        fields.insert("id".to_owned(), Value::from(id));
        fields.insert("gId".to_owned(), Value::from(gid));
        let world = self.file.group_mut(WORLD_OBJECTS);
        world.push(Object::new(fields));
        self.world_objects.insert(id, world.objects().len() - 1);

        let inventory = &mut self.file.group_mut(INVENTORIES).objects_mut()[index];
        let mut ids = wo_ids(inventory);
        ids.push(id);
        inventory.set("woIds", Value::from(join(&ids)));
        Some(id)
    }

    /// Removes the first `world_object_id` from the inventory's `woIds`. The
    /// world object itself stays, as in .NET. Returns whether it was there.
    pub fn remove_item(&mut self, inventory_id: i64, world_object_id: i64) -> bool {
        let Some(index) = self.inventory_index(inventory_id) else {
            return false;
        };
        let inventory = &mut self.file.group_mut(INVENTORIES).objects_mut()[index];
        let mut ids = wo_ids(inventory);
        let Some(position) = ids.iter().position(|id| *id == world_object_id) else {
            return false;
        };
        ids.remove(position);
        inventory.set("woIds", Value::from(join(&ids)))
    }

    /// Distinct `gId`s of all world objects, sorted.
    pub fn gids(&self) -> Vec<String> {
        self.file
            .group(WORLD_OBJECTS)
            .objects()
            .iter()
            .filter_map(|o| o.get("gId").and_then(Value::as_str))
            .filter(|gid| !gid.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    fn inventory(&self, id: i64) -> Option<&Object> {
        self.inventory_index(id)
            .map(|index| &self.file.group(INVENTORIES).objects()[index])
    }

    fn inventory_index(&self, id: i64) -> Option<usize> {
        self.file
            .group(INVENTORIES)
            .objects()
            .iter()
            .position(|inventory| int(inventory.get("id")) == Some(id))
    }
}

/// An integer field, also when stored as a whole float (`12.0`).
pub fn int(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    value.as_i64().or_else(|| {
        let float = value.as_f64()?;
        #[allow(clippy::cast_possible_truncation)] // whole and in range
        (float.fract() == 0.0 && float.abs() <= 9_007_199_254_740_992.0).then_some(float as i64)
    })
}

/// A string field, or `""`.
pub fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or_default()
}

/// The ids in an inventory's `woIds`, split on `,` and `|`; entries that are
/// not non-negative integers are dropped.
fn wo_ids(inventory: &Object) -> Vec<i64> {
    text(inventory.get("woIds"))
        .split([',', '|'])
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .filter(|id| *id >= 0)
        .collect()
}

fn join(ids: &[i64]) -> String {
    ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}
