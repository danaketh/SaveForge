//! What a savegame says about itself: `SaveInfo.json` and `meta.lsf` inside
//! the `.lsv` package.

use std::fmt::Write as _;

use lslib::lsf::LsfFile;
use lslib::package::Package;
use lslib::resource::{AttributeValue, Node};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveSummary {
    pub save_name: String,
    /// The player character's name (`meta.lsf` `LeaderName`).
    pub leader: Option<String>,
    pub game_version: String,
    pub level: String,
    /// Difficulty and ruleset ids, such as `DifficultyHard`, `RulesetLarian`.
    pub difficulty: Vec<String>,
    pub party: Vec<PartyMember>,
    /// Display names of the modules the save was made with.
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartyMember {
    /// `Generic` for a custom character, else the origin's name.
    pub origin: String,
    pub race: String,
    /// `Main` or `Main (Sub)` per class, joined by `, `.
    pub classes: String,
    pub level: Option<u64>,
}

impl SaveSummary {
    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let package =
            Package::read(bytes).map_err(|e| format!("not a Baldur's Gate 3 save: {e}"))?;
        let info = package
            .find_ignore_case("SaveInfo.json")
            .ok_or("the save has no SaveInfo.json")?;
        let info = package
            .contents(info)
            .map_err(|e| format!("cannot read SaveInfo.json: {e}"))?;
        let info: Value = serde_json::from_slice(&info)
            .map_err(|e| format!("SaveInfo.json is not valid JSON: {e}"))?;
        let mut summary = Self::from_save_info(&info);

        if let Some(meta) = package.find_ignore_case("meta.lsf") {
            let meta = package
                .contents(meta)
                .and_then(|bytes| LsfFile::read(&bytes))
                .map_err(|e| format!("cannot read meta.lsf: {e}"))?;
            if let Some(meta) = meta
                .resource
                .region("MetaData")
                .and_then(|r| r.child("MetaData"))
            {
                summary.read_meta(meta);
            }
        }
        Ok(summary)
    }

    fn from_save_info(info: &Value) -> Self {
        let party = info["Active Party"]["Characters"]
            .as_array()
            .map(|members| members.iter().map(PartyMember::from_json).collect())
            .unwrap_or_default();
        Self {
            save_name: string(&info["Save Name"]),
            leader: None,
            game_version: string(&info["Game Version"]),
            level: string(&info["Current Level"]),
            difficulty: info["Difficulty"]
                .as_array()
                .map(|ids| {
                    ids.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            party,
            mods: Vec::new(),
        }
    }

    fn read_meta(&mut self, meta: &Node) {
        self.leader = attribute_str(meta, "LeaderName")
            .filter(|name| !name.is_empty())
            .map(str::to_owned);
        self.mods = meta
            .child("ModuleSettings")
            .and_then(|settings| settings.child("Mods"))
            .map(|mods| {
                mods.children_named("ModuleShortDesc")
                    .filter_map(|module| attribute_str(module, "Name"))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
    }
}

impl PartyMember {
    fn from_json(member: &Value) -> Self {
        let classes = member["Classes"]
            .as_array()
            .map(|classes| {
                classes.iter().fold(String::new(), |mut out, class| {
                    if !out.is_empty() {
                        out.push_str(", ");
                    }
                    out.push_str(class["Main"].as_str().unwrap_or_default());
                    if let Some(sub) = class["Sub"].as_str().filter(|s| !s.is_empty()) {
                        let _ = write!(out, " ({sub})");
                    }
                    out
                })
            })
            .unwrap_or_default();
        Self {
            origin: string(&member["Origin"]),
            race: string(&member["Race"]),
            classes,
            level: member["Level"].as_u64(),
        }
    }
}

fn string(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn attribute_str<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attribute(name).and_then(AttributeValue::as_str)
}
