//! Local editor state and byte-preserving lighting edits recovered from OemDrv.exe.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub name: String,
    pub identity: [u8; 6],
    pub keys: Vec<KeySpec>,
    pub effects: Vec<EffectSpec>,
    pub sleep_seconds: Vec<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeySpec {
    pub id: usize,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub virtual_key: u8,
    pub matrix_index: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectSpec {
    pub slot: usize,
    pub hardware_id: u8,
    pub label: String,
    pub has_speed: bool,
    pub has_brightness: bool,
    pub has_direction: bool,
    pub has_random: bool,
    pub has_color: bool,
}
impl Catalogue {
    pub fn bundled() -> Result<Self> {
        let cat: Self = serde_json::from_str(include_str!("../assets/catalogue.json"))?;
        ensure!(
            !cat.keys.is_empty() && !cat.effects.is_empty(),
            "Empty catalogue"
        );
        Ok(cat)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Binding {
    Key { usage: u8, modifiers: u8 },
    Media { usage: u16 },
    Disabled,
    Macro { index: usize },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroEvent {
    pub usage: u8,
    pub pressed: bool,
    pub delay_ms: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Macro {
    pub name: String,
    pub events: Vec<MacroEvent>,
    pub repeat: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub effect_slot: usize,
    pub brightness: u8,
    pub speed: u8,
    pub color: u8,
    pub sleep_seconds: u16,
    pub debounce_ms: u8,
    pub remaps: BTreeMap<String, Binding>,
    pub macros: Vec<Macro>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub version: u32,
    pub selected: usize,
    pub profiles: Vec<Profile>,
}
impl Workspace {
    pub fn new(cat: &Catalogue) -> Self {
        Self {
            version: 1,
            selected: 0,
            profiles: vec![Profile {
                name: "Default".into(),
                effect_slot: 0,
                brightness: 9,
                speed: 4,
                color: 7,
                sleep_seconds: cat.sleep_seconds.first().copied().unwrap_or(300),
                debounce_ms: 1,
                remaps: BTreeMap::new(),
                macros: Vec::new(),
            }],
        }
    }
    pub fn current(&self) -> &Profile {
        &self.profiles[self.selected]
    }
    pub fn current_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.selected]
    }
    pub fn validate(&self, cat: &Catalogue) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported profile format version {}",
            self.version
        );
        ensure!(
            !self.profiles.is_empty() && self.selected < self.profiles.len(),
            "Invalid selected profile"
        );
        for p in &self.profiles {
            ensure!(!p.name.trim().is_empty(), "Profile name is empty");
            ensure!(p.effect_slot < cat.effects.len(), "Unknown effect slot");
            ensure!(
                p.brightness <= 9 && p.speed <= 4 && p.color <= 7,
                "Lighting value out of range"
            );
            ensure!(
                cat.sleep_seconds.contains(&p.sleep_seconds),
                "Unsupported sleep time"
            );
            for (key, binding) in &p.remaps {
                let (layer, id) = key
                    .split_once(':')
                    .context("Invalid remap key identifier")?;
                let layer: usize = layer.parse()?;
                let id: usize = id.parse()?;
                ensure!(
                    layer < 4 && cat.keys.iter().any(|k| k.id == id),
                    "Invalid remap target"
                );
                match binding {
                    Binding::Key { usage, .. } => {
                        ensure!(*usage > 0, "HID key usage must not be zero")
                    }
                    Binding::Macro { index } => {
                        ensure!(*index < p.macros.len(), "Missing macro reference")
                    }
                    _ => (),
                }
            }
            for m in &p.macros {
                ensure!(
                    !m.name.trim().is_empty() && m.repeat > 0,
                    "Invalid macro name/repeat"
                );
                for e in &m.events {
                    ensure!(e.usage > 0 && e.delay_ms <= 0xfffff, "Invalid macro event");
                }
            }
        }
        Ok(())
    }
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    pub fn from_json(json: &str, cat: &Catalogue) -> Result<Self> {
        let ws: Self =
            serde_json::from_str(json).context("Not a native AULA Linux workspace JSON")?;
        ws.validate(cat)?;
        Ok(ws)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate(&Catalogue::bundled()?)?;
        let parent = path.parent().context("Profile path has no parent")?;
        fs::create_dir_all(parent)?;
        let temp = path.with_extension(format!("json.{}.new", std::process::id()));
        let mut opts = fs::OpenOptions::new();
        use std::io::Write;
        let mut f = opts.write(true).create_new(true).open(&temp)?;
        let result = (|| -> Result<()> {
            f.write_all(self.to_json()?.as_bytes())?;
            f.sync_all()?;
            fs::rename(&temp, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
    pub fn load(path: &Path, cat: &Catalogue) -> Result<Self> {
        Self::from_json(&fs::read_to_string(path)?, cat)
    }
}
impl Profile {
    /// Preserve all unedited bytes. Synthetic defaults are never a device profile.
    /// Source: 0x48d680 + keyboard-specific loader 0x410a70, docs/re/lighting-audit.md.
    pub fn patch_lighting(&self, original: &[u8], cat: &Catalogue) -> Result<[u8; 128]> {
        ensure!(
            original.len() == 128 && original[126..] == [0x5a, 0xa5],
            "Invalid/unread device profile"
        );
        ensure!(self.effect_slot < cat.effects.len(), "Unknown effect slot");
        ensure!(
            self.brightness <= 9 && self.speed <= 4 && self.color <= 7,
            "Lighting value out of range"
        );
        // Custom colors require a separately reconstructed matrix operation.
        if self.effect_slot == 18 {
            bail!("Custom per-key hardware colors are not validated yet");
        }
        let mut patched: [u8; 128] = original.try_into()?;
        patched[9] = 0;
        patched[10] = cat.effects[self.effect_slot].hardware_id;
        let pair = 0x3a + self.effect_slot * 2;
        ensure!(pair + 1 < 126, "Effect parameters exceed profile");
        patched[pair] = self.brightness;
        patched[pair + 1] = (self.speed << 4) | self.color;
        Ok(patched)
    }
}
