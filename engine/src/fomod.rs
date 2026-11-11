//! FOMOD installers: the `fomod/ModuleConfig.xml` (and `fomod/info.xml`) many Bethesda mods ship with, so the
//! player picks options and only the chosen files are installed. Nothing in the archive is ever run; this only reads
//! the XML and works out which files go where.
//!
//! Written from the public FOMOD format (the XmlScript 5.0 schema) and how installers are known to behave:
//! - Steps, groups and options are shown sorted by name unless their `order` says "Explicit" (the schema's default
//!   for `order` is "Ascending").
//! - A `file` or `folder` without a `destination` goes to the same path as its `source`.
//! - `priority` defaults to 0; files from the required list go in first, then the chosen options' files, then the
//!   conditional installs, each phase sorted by priority, and a later file wins over an earlier one at the same path.
//! - A flag condition with an empty value matches a flag that was never set; otherwise values must match exactly.
//! - `alwaysInstall` files go in even when their option isn't chosen; `installIfUsable` ones when the option isn't
//!   "NotUsable".
//!
//! Paths inside the archive are matched ignoring case, with '\' or '/' separators. Destinations are relative to the
//! game's mod folder (Data for Bethesda games).
//!
//! The app drives a `Session` step by step: `current()` shows a step, `select`/`deselect`/`toggle` change options,
//! `next`/`back` move, and `finish` returns the files to install.

use crate::json::quote;
use crate::xml::{self, Element};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    ExactlyOne,
    AtMostOne,
    AtLeastOne,
    All,
    Any,
}

impl GroupKind {
    fn parse(s: &str) -> GroupKind {
        match s.trim().to_ascii_lowercase().as_str() {
            "selectexactlyone" => GroupKind::ExactlyOne,
            "selectatmostone" => GroupKind::AtMostOne,
            "selectatleastone" => GroupKind::AtLeastOne,
            "selectall" => GroupKind::All,
            _ => GroupKind::Any,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            GroupKind::ExactlyOne => "exactlyOne",
            GroupKind::AtMostOne => "atMostOne",
            GroupKind::AtLeastOne => "atLeastOne",
            GroupKind::All => "all",
            GroupKind::Any => "any",
        }
    }

    /// Plain words for the group's rule, shown under its name.
    pub fn hint(self) -> &'static str {
        match self {
            GroupKind::ExactlyOne => "Pick one",
            GroupKind::AtMostOne => "Pick one or none",
            GroupKind::AtLeastOne => "Pick at least one",
            GroupKind::All => "All of these are installed",
            GroupKind::Any => "Pick any",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    Required,
    Optional,
    Recommended,
    NotUsable,
    CouldBeUsable,
}

impl PluginType {
    fn parse(s: &str) -> PluginType {
        match s.trim().to_ascii_lowercase().as_str() {
            "required" => PluginType::Required,
            "recommended" => PluginType::Recommended,
            "notusable" => PluginType::NotUsable,
            "couldbeusable" => PluginType::CouldBeUsable,
            _ => PluginType::Optional,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            PluginType::Required => "required",
            PluginType::Optional => "optional",
            PluginType::Recommended => "recommended",
            PluginType::NotUsable => "notUsable",
            PluginType::CouldBeUsable => "couldBeUsable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Active,
    Inactive,
    Missing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    File { file: String, state: FileState },
    Flag { flag: String, value: String },
    Game { version: String },
    /// The old installer's own version: always met.
    Fomm,
    All(Vec<Condition>),
    Any(Vec<Condition>),
}

/// One `file` or `folder` entry.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallItem {
    pub source: String,
    /// None when the attribute is missing (then it's the same as `source`).
    pub destination: Option<String>,
    pub folder: bool,
    pub priority: i64,
    pub always: bool,
    pub if_usable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDescriptor {
    pub default: PluginType,
    /// First pattern whose conditions are met decides the type.
    pub patterns: Vec<(Condition, PluginType)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    /// Image path inside the archive (relative to the installer's root), as written.
    pub image: Option<String>,
    pub files: Vec<InstallItem>,
    pub flags: Vec<(String, String)>,
    pub kind: TypeDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub kind: GroupKind,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub name: String,
    pub visible: Option<Condition>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub module_name: String,
    pub module_image: Option<String>,
    /// Conditions the whole mod needs (shown as a warning when not met; installing is still allowed).
    pub dependencies: Option<Condition>,
    pub required: Vec<InstallItem>,
    pub steps: Vec<Step>,
    pub conditional: Vec<(Condition, Vec<InstallItem>)>,
}

/// What `fomod/info.xml` says about the mod.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Info {
    pub name: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------------------------------------------
// Finding and reading
// ---------------------------------------------------------------------------------------------------------------

/// Where an installer sits in an archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The folder holding `fomod/` ("" or "Wrapper/"), with a trailing '/' when not empty. Paths in the installer
    /// are relative to it.
    pub root: String,
    /// The ModuleConfig.xml's path in the archive, if there is one.
    pub config: Option<String>,
    pub info: Option<String>,
}

/// Looks for `fomod/ModuleConfig.xml` (any case) at the top of the archive or under wrapper folders.
/// The shallowest one wins.
pub fn find(files: &[String]) -> Option<Found> {
    let mut best: Option<(usize, Found)> = None;
    for f in files {
        let clean = f.replace('\\', "/");
        let parts: Vec<&str> = clean.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() < 2 || !parts[parts.len() - 2].eq_ignore_ascii_case("fomod") {
            continue;
        }
        let last = parts[parts.len() - 1].to_ascii_lowercase();
        if last != "moduleconfig.xml" && last != "info.xml" {
            continue;
        }
        let depth = parts.len() - 2;
        let root = if depth == 0 { String::new() } else { format!("{}/", parts[..depth].join("/")) };
        let entry = match &mut best {
            Some((d, found)) if *d == depth && found.root.eq_ignore_ascii_case(&root) => found,
            Some((d, _)) if *d <= depth => continue,
            _ => {
                best = Some((depth, Found { root: root.clone(), config: None, info: None }));
                &mut best.as_mut().unwrap().1
            }
        };
        if last == "moduleconfig.xml" {
            entry.config = Some(f.clone());
        } else {
            entry.info = Some(f.clone());
        }
    }
    best.map(|(_, f)| f)
}

fn order_of(e: Option<&Element>) -> &str {
    e.and_then(|e| e.attr("order")).unwrap_or("Ascending")
}

fn sort_named<T>(items: &mut [T], order: &str, name: impl Fn(&T) -> String) {
    match order.trim().to_ascii_lowercase().as_str() {
        "explicit" => {}
        "descending" => items.sort_by_key(|b| std::cmp::Reverse(name(b).to_lowercase())),
        _ => items.sort_by_key(|a| name(a).to_lowercase()),
    }
}

fn parse_bool(s: Option<&str>) -> bool {
    matches!(s.map(|s| s.trim().to_ascii_lowercase()).as_deref(), Some("true" | "1"))
}

fn parse_items(e: Option<&Element>) -> Vec<InstallItem> {
    let Some(e) = e else { return Vec::new() };
    e.elements()
        .filter(|c| c.is("file") || c.is("folder"))
        .map(|c| InstallItem {
            source: c.attr("source").unwrap_or("").to_string(),
            destination: c.attr("destination").map(String::from),
            folder: c.is("folder"),
            priority: c.attr("priority").and_then(|p| p.trim().parse::<i64>().ok()).unwrap_or(0),
            always: parse_bool(c.attr("alwaysInstall")),
            if_usable: parse_bool(c.attr("installIfUsable")),
        })
        .collect()
}

/// Reads a composite condition (`dependencies`, `visible`, `moduleDependencies`): its own `operator` and children.
fn parse_composite(e: &Element) -> Condition {
    let items: Vec<Condition> = e.elements().filter_map(parse_condition).collect();
    if e.attr("operator").is_some_and(|o| o.trim().eq_ignore_ascii_case("or")) {
        Condition::Any(items)
    } else {
        Condition::All(items)
    }
}

fn parse_condition(e: &Element) -> Option<Condition> {
    if e.is("fileDependency") {
        let state = match e.attr("state").unwrap_or("Active").trim().to_ascii_lowercase().as_str() {
            "inactive" => FileState::Inactive,
            "missing" => FileState::Missing,
            _ => FileState::Active,
        };
        Some(Condition::File { file: e.attr("file").unwrap_or("").to_string(), state })
    } else if e.is("flagDependency") {
        Some(Condition::Flag { flag: e.attr("flag").unwrap_or("").to_string(), value: e.attr("value").unwrap_or("").to_string() })
    } else if e.is("gameDependency") {
        Some(Condition::Game { version: e.attr("version").unwrap_or("").to_string() })
    } else if e.is("fommDependency") || e.is("foseDependency") || e.is("nvseDependency") {
        Some(Condition::Fomm)
    } else if e.is("dependencies") {
        Some(parse_composite(e))
    } else {
        None
    }
}

fn parse_type(e: Option<&Element>) -> TypeDescriptor {
    let mut td = TypeDescriptor { default: PluginType::Optional, patterns: Vec::new() };
    let Some(e) = e else { return td };
    if let Some(t) = e.child("type") {
        td.default = PluginType::parse(t.attr("name").unwrap_or(""));
    } else if let Some(dt) = e.child("dependencyType") {
        if let Some(d) = dt.child("defaultType") {
            td.default = PluginType::parse(d.attr("name").unwrap_or(""));
        }
        if let Some(patterns) = dt.child("patterns") {
            for p in patterns.children_named("pattern") {
                let cond = p.child("dependencies").map(parse_composite).unwrap_or(Condition::All(Vec::new()));
                let kind = PluginType::parse(p.child("type").and_then(|t| t.attr("name")).unwrap_or(""));
                td.patterns.push((cond, kind));
            }
        }
    }
    td
}

fn parse_plugin(e: &Element) -> Plugin {
    let flags = e
        .child("conditionFlags")
        .map(|f| f.children_named("flag").map(|fl| (fl.attr("name").unwrap_or("").to_string(), fl.text())).collect())
        .unwrap_or_default();
    Plugin {
        name: e.attr("name").unwrap_or("").trim().to_string(),
        description: e.child("description").map(|d| d.text()).unwrap_or_default(),
        image: e.child("image").and_then(|i| i.attr("path")).filter(|p| !p.trim().is_empty()).map(String::from),
        files: parse_items(e.child("files")),
        flags,
        kind: parse_type(e.child("typeDescriptor")),
    }
}

/// Reads ModuleConfig.xml (raw bytes: UTF-8 or UTF-16, with or without BOM).
pub fn parse(bytes: &[u8]) -> Result<Config, String> {
    let root = xml::parse_bytes(bytes).map_err(|e| format!("its installer file can't be read: {e}"))?;
    if !root.is("config") {
        return Err(format!("its installer file isn't a FOMOD installer (it starts with <{}>)", root.name));
    }
    let mut cfg = Config {
        module_name: root.child("moduleName").map(|m| m.text()).unwrap_or_default(),
        module_image: root.child("moduleImage").and_then(|i| i.attr("path")).filter(|p| !p.trim().is_empty()).map(String::from),
        dependencies: root.child("moduleDependencies").map(parse_composite),
        required: parse_items(root.child("requiredInstallFiles")),
        ..Config::default()
    };
    if let Some(steps_el) = root.child("installSteps") {
        let mut steps: Vec<Step> = steps_el
            .children_named("installStep")
            .map(|s| {
                let groups_el = s.child("optionalFileGroups");
                let mut groups: Vec<Group> = groups_el
                    .map(|g| g.children_named("group").collect::<Vec<_>>())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|g| {
                        let plugins_el = g.child("plugins");
                        let mut plugins: Vec<Plugin> =
                            plugins_el.map(|p| p.children_named("plugin").map(parse_plugin).collect()).unwrap_or_default();
                        sort_named(&mut plugins, order_of(plugins_el), |p| p.name.clone());
                        Group { name: g.attr("name").unwrap_or("").trim().to_string(), kind: GroupKind::parse(g.attr("type").unwrap_or("")), plugins }
                    })
                    .collect();
                sort_named(&mut groups, order_of(groups_el), |g| g.name.clone());
                Step { name: s.attr("name").unwrap_or("").trim().to_string(), visible: s.child("visible").map(parse_composite), groups }
            })
            .collect();
        sort_named(&mut steps, order_of(Some(steps_el)), |s| s.name.clone());
        cfg.steps = steps;
    }
    if let Some(c) = root.child("conditionalFileInstalls") && let Some(patterns) = c.child("patterns") {
        for p in patterns.children_named("pattern") {
            let cond = p.child("dependencies").map(parse_composite).unwrap_or(Condition::All(Vec::new()));
            cfg.conditional.push((cond, parse_items(p.child("files"))));
        }
    }
    Ok(cfg)
}

/// Reads fomod/info.xml. Never fails: an unreadable file gives an empty Info.
pub fn parse_info(bytes: &[u8]) -> Info {
    let Ok(root) = xml::parse_bytes(bytes) else { return Info::default() };
    let field = |n: &str| root.child(n).map(|e| e.text()).filter(|t| !t.is_empty());
    Info {
        name: field("Name"),
        author: field("Author"),
        version: field("Version"),
        website: field("Website"),
        description: field("Description"),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------------------------------------------

/// What conditions can look at besides flags: the plugins the player has (lowercase file name -> state) and the game
/// version, when known.
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub files: HashMap<String, FileState>,
    pub game_version: Option<String>,
}

impl Env {
    /// Builds the plugin states from (file name, active) pairs.
    pub fn with_plugins(plugins: &[(String, bool)]) -> Env {
        Env {
            files: plugins
                .iter()
                .map(|(n, on)| (n.to_lowercase(), if *on { FileState::Active } else { FileState::Inactive }))
                .collect(),
            game_version: None,
        }
    }
}

fn version_at_least(have: &str, need: &str) -> bool {
    let nums = |s: &str| s.split(['.', ',', ' ']).map(|p| p.trim().parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (a, b) = (nums(have), nums(need));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    true
}

fn met(c: &Condition, flags: &HashMap<String, String>, env: &Env) -> bool {
    match c {
        Condition::File { file, state } => {
            let name = file.replace('\\', "/");
            let name = name.rsplit('/').next().unwrap_or("").to_lowercase();
            let have = env.files.get(&name).copied().unwrap_or(FileState::Missing);
            have == *state
        }
        Condition::Flag { flag, value } => {
            let have = flags.get(flag).map(String::as_str).unwrap_or("");
            if value.is_empty() {
                have.is_empty()
            } else {
                have == value
            }
        }
        Condition::Game { version } => env.game_version.as_deref().is_none_or(|v| version.trim().is_empty() || version_at_least(v, version)),
        Condition::Fomm => true,
        Condition::All(items) => items.iter().all(|c| met(c, flags, env)),
        Condition::Any(items) => items.is_empty() || items.iter().any(|c| met(c, flags, env)),
    }
}

impl TypeDescriptor {
    fn resolve(&self, flags: &HashMap<String, String>, env: &Env) -> PluginType {
        self.patterns.iter().find(|(c, _)| met(c, flags, env)).map(|(_, t)| *t).unwrap_or(self.default)
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct OptionView {
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub kind: PluginType,
    pub selected: bool,
    /// Can the player change it (false for required, not usable, and "all" groups).
    pub can_change: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupView {
    pub name: String,
    pub kind: GroupKind,
    pub options: Vec<OptionView>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepView {
    /// Index in the installer's step list.
    pub index: usize,
    pub name: String,
    /// 1 for the first step shown, counting the steps visited so far.
    pub number: usize,
    pub groups: Vec<GroupView>,
    /// No more steps after this one (with the choices as they are now): the next press installs.
    pub is_last: bool,
    pub can_go_back: bool,
}

/// The result of an installer: archive path (real casing) -> destination relative to the mod folder.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Finish {
    pub files: Vec<(String, String)>,
    /// Sources the installer names that aren't in the archive (installed without them).
    pub missing: Vec<String>,
    /// The flags set by the choices, for the activity log.
    pub flags: Vec<(String, String)>,
}

/// A running installer.
#[derive(Debug, Clone)]
pub struct Session {
    config: Config,
    env: Env,
    root: String,
    /// Lowercase archive path -> real archive path (files only).
    files: HashMap<String, String>,
    /// Lowercase archive paths, sorted, for folder lookups.
    sorted: Vec<(String, String)>,
    /// [step][group][plugin]
    sel: Vec<Vec<Vec<bool>>>,
    ready: Vec<bool>,
    history: Vec<usize>,
    current: Option<usize>,
    /// Earlier choices ("step/group/option") to start from instead of the defaults.
    preferred: Option<HashSet<String>>,
}

fn choice_key(step: &str, group: &str, option: &str) -> String {
    format!("{}/{}/{}", step.to_lowercase(), group.to_lowercase(), option.to_lowercase())
}

/// Cleans an installer path: '/' separators, no empty or "." parts. Returns None for ".." or absolute paths.
fn clean_path(p: &str) -> Option<String> {
    let p = p.replace('\\', "/");
    let p = p.trim();
    if p.starts_with('/') || p.chars().nth(1) == Some(':') {
        return None;
    }
    let mut parts = Vec::new();
    for part in p.split('/') {
        match part.trim() {
            "" | "." => {}
            ".." => return None,
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

fn join(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a}/{b}"),
    }
}

impl Session {
    /// Starts an installer. `archive_files` are the archive's file paths ('/' separated); `root` is `Found::root`.
    pub fn new(config: Config, env: Env, archive_files: &[String], root: &str) -> Session {
        let files: HashMap<String, String> =
            archive_files.iter().map(|f| f.replace('\\', "/")).map(|f| (f.to_lowercase(), f)).collect();
        let mut sorted: Vec<(String, String)> = files.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        sorted.sort();
        let sel = config.steps.iter().map(|s| s.groups.iter().map(|g| vec![false; g.plugins.len()]).collect()).collect();
        let ready = vec![false; config.steps.len()];
        let mut s = Session {
            config,
            env,
            root: root.replace('\\', "/").trim_matches('/').to_string(),
            files,
            sorted,
            sel,
            ready,
            history: Vec::new(),
            current: None,
            preferred: None,
        };
        s.current = s.next_visible(0);
        if let Some(c) = s.current {
            s.enter(c);
        }
        s
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The mod's name as the installer gives it.
    pub fn module_name(&self) -> &str {
        &self.config.module_name
    }

    /// The installer has steps with choices (false: it only has fixed files; `finish` right away).
    pub fn has_choices(&self) -> bool {
        self.current.is_some()
    }

    /// Starts from earlier choices (from `choices()` of an earlier install of the same mod) instead of the defaults,
    /// where they still fit. Applies to the current step and every step entered later.
    pub fn apply_choices(&mut self, choices: &[String]) {
        self.preferred = Some(choices.iter().map(|c| c.to_lowercase()).collect());
        if let Some(c) = self.current {
            self.ready[c] = false;
            self.enter(c);
        }
    }

    /// The choices made, as "step/group/option" (lowercase), for `apply_choices` next time.
    pub fn choices(&self) -> Vec<String> {
        let mut out = Vec::new();
        for &si in self.path().iter() {
            let step = &self.config.steps[si];
            for (gi, g) in step.groups.iter().enumerate() {
                for (pi, p) in g.plugins.iter().enumerate() {
                    if self.sel[si][gi][pi] {
                        out.push(choice_key(&step.name, &g.name, &p.name));
                    }
                }
            }
        }
        out
    }

    /// Steps taken so far, including the current one.
    fn path(&self) -> Vec<usize> {
        let mut p = self.history.clone();
        p.extend(self.current);
        p
    }

    /// Flags set by the chosen options of these steps, in order.
    fn flags_of(&self, steps: &[usize]) -> HashMap<String, String> {
        let mut flags = HashMap::new();
        for &si in steps {
            for (gi, g) in self.config.steps[si].groups.iter().enumerate() {
                for (pi, p) in g.plugins.iter().enumerate() {
                    if self.sel[si][gi][pi] {
                        for (k, v) in &p.flags {
                            flags.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }
        flags
    }

    /// Flags before the current step.
    fn flags_before(&self) -> HashMap<String, String> {
        self.flags_of(&self.history)
    }

    /// The first visible step at or after `from`, with the flags of the path so far (including the current step).
    fn next_visible(&self, from: usize) -> Option<usize> {
        let flags = self.flags_of(&self.path());
        (from..self.config.steps.len())
            .find(|&i| self.config.steps[i].visible.as_ref().is_none_or(|c| met(c, &flags, &self.env)))
    }

    fn kind_of(&self, si: usize, gi: usize, pi: usize, flags: &HashMap<String, String>) -> PluginType {
        self.config.steps[si].groups[gi].plugins[pi].kind.resolve(flags, &self.env)
    }

    /// Sets up a step's choices when it is shown: defaults the first time (or earlier choices), then the rules.
    fn enter(&mut self, si: usize) {
        let flags = self.flags_before();
        let step = self.config.steps[si].clone();
        for (gi, g) in step.groups.iter().enumerate() {
            let kinds: Vec<PluginType> = (0..g.plugins.len()).map(|pi| self.kind_of(si, gi, pi, &flags)).collect();
            let row = &mut self.sel[si][gi];
            if !self.ready[si] {
                let preferred: Vec<bool> = match &self.preferred {
                    Some(p) => g.plugins.iter().map(|pl| p.contains(&choice_key(&step.name, &g.name, &pl.name))).collect(),
                    None => vec![false; g.plugins.len()],
                };
                if preferred.iter().any(|&b| b) {
                    *row = preferred;
                } else {
                    *row = kinds.iter().map(|k| matches!(k, PluginType::Required | PluginType::Recommended)).collect();
                }
            }
            // The rules always hold: required on, not usable off, group limits.
            for (pi, k) in kinds.iter().enumerate() {
                match k {
                    PluginType::Required => row[pi] = true,
                    PluginType::NotUsable => row[pi] = false,
                    _ => {}
                }
            }
            let first_usable = kinds.iter().position(|k| *k != PluginType::NotUsable);
            match g.kind {
                GroupKind::All => row.iter_mut().for_each(|b| *b = true),
                GroupKind::ExactlyOne | GroupKind::AtMostOne => {
                    let keep = row.iter().position(|&b| b);
                    row.iter_mut().for_each(|b| *b = false);
                    match keep {
                        Some(k) => row[k] = true,
                        None if g.kind == GroupKind::ExactlyOne => {
                            if let Some(f) = first_usable.or(if row.is_empty() { None } else { Some(0) }) {
                                row[f] = true;
                            }
                        }
                        None => {}
                    }
                }
                GroupKind::AtLeastOne => {
                    if !row.iter().any(|&b| b) && let Some(f) = first_usable.or(if row.is_empty() { None } else { Some(0) }) {
                        row[f] = true;
                    }
                }
                GroupKind::Any => {}
            }
        }
        self.ready[si] = true;
    }

    /// The step being shown, or None when the installer has no steps (or they are all hidden).
    pub fn current(&self) -> Option<StepView> {
        let si = self.current?;
        let flags = self.flags_before();
        let step = &self.config.steps[si];
        let groups = step
            .groups
            .iter()
            .enumerate()
            .map(|(gi, g)| GroupView {
                name: g.name.clone(),
                kind: g.kind,
                options: g
                    .plugins
                    .iter()
                    .enumerate()
                    .map(|(pi, p)| {
                        let kind = self.kind_of(si, gi, pi, &flags);
                        let can_change = g.kind != GroupKind::All && !matches!(kind, PluginType::Required | PluginType::NotUsable);
                        OptionView {
                            name: p.name.clone(),
                            description: p.description.clone(),
                            image: p.image.clone(),
                            kind,
                            selected: self.sel[si][gi][pi],
                            can_change,
                        }
                    })
                    .collect(),
            })
            .collect();
        Some(StepView {
            index: si,
            name: step.name.clone(),
            number: self.history.len() + 1,
            groups,
            is_last: self.next_visible(si + 1).is_none(),
            can_go_back: !self.history.is_empty(),
        })
    }

    fn locate(&self, group: usize, option: usize) -> Result<(usize, PluginType, GroupKind), String> {
        let si = self.current.ok_or("There is no step to choose in.")?;
        let g = self.config.steps[si].groups.get(group).ok_or("That group isn't on this step.")?;
        if option >= g.plugins.len() {
            return Err("That option isn't in this group.".into());
        }
        let kind = self.kind_of(si, group, option, &self.flags_before());
        Ok((si, kind, g.kind))
    }

    /// Chooses an option. In "pick one" groups this replaces the other choice.
    pub fn select(&mut self, group: usize, option: usize) -> Result<(), String> {
        let (si, kind, gk) = self.locate(group, option)?;
        if kind == PluginType::NotUsable {
            return Err("This option can't be used with your current setup.".into());
        }
        if matches!(gk, GroupKind::ExactlyOne | GroupKind::AtMostOne) {
            let flags = self.flags_before();
            // Never take away a required option to pick another.
            for (pi, on) in self.sel[si][group].iter().enumerate() {
                if *on && pi != option && self.kind_of(si, group, pi, &flags) == PluginType::Required {
                    return Err("Another option in this group is required, so this one can't be picked.".into());
                }
            }
            let row = &mut self.sel[si][group];
            row.iter_mut().for_each(|b| *b = false);
            row[option] = true;
        } else {
            self.sel[si][group][option] = true;
        }
        Ok(())
    }

    /// Unchooses an option, where the group allows it.
    pub fn deselect(&mut self, group: usize, option: usize) -> Result<(), String> {
        let (si, kind, gk) = self.locate(group, option)?;
        if !self.sel[si][group][option] {
            return Ok(());
        }
        if kind == PluginType::Required {
            return Err("This option is required.".into());
        }
        match gk {
            GroupKind::All => return Err("Everything in this group is installed.".into()),
            GroupKind::ExactlyOne => return Err("This group needs one option: pick another one instead.".into()),
            GroupKind::AtLeastOne if self.sel[si][group].iter().filter(|&&b| b).count() <= 1 => {
                return Err("This group needs at least one option.".into())
            }
            _ => {}
        }
        self.sel[si][group][option] = false;
        Ok(())
    }

    /// Selects or unselects (what pressing A on an option does).
    pub fn toggle(&mut self, group: usize, option: usize) -> Result<(), String> {
        let (si, _, gk) = self.locate(group, option)?;
        if self.sel[si][group][option] && gk != GroupKind::ExactlyOne {
            self.deselect(group, option)
        } else {
            self.select(group, option)
        }
    }

    /// Checks the current step's groups are complete.
    fn check_current(&self) -> Result<(), String> {
        let Some(si) = self.current else { return Ok(()) };
        for (gi, g) in self.config.steps[si].groups.iter().enumerate() {
            let n = self.sel[si][gi].iter().filter(|&&b| b).count();
            let ok = match g.kind {
                GroupKind::ExactlyOne => n == 1 || g.plugins.is_empty(),
                GroupKind::AtLeastOne => n >= 1 || g.plugins.is_empty(),
                GroupKind::AtMostOne => n <= 1,
                _ => true,
            };
            if !ok {
                let name = if g.name.is_empty() { "A group".to_string() } else { format!("\"{}\"", g.name) };
                return Err(format!("{name} needs a choice: {}.", g.kind.hint().to_lowercase()));
            }
        }
        Ok(())
    }

    /// Goes to the next step. Returns false when this was the last one (then call `finish`).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<bool, String> {
        self.check_current()?;
        let Some(si) = self.current else { return Ok(false) };
        let Some(n) = self.next_visible(si + 1) else { return Ok(false) };
        self.history.push(si);
        self.current = Some(n);
        self.enter(n);
        Ok(true)
    }

    /// Goes back a step (choices made there are kept). Returns false at the first step.
    pub fn back(&mut self) -> bool {
        match self.history.pop() {
            Some(prev) => {
                self.current = Some(prev);
                self.enter(prev);
                true
            }
            None => false,
        }
    }

    /// Does the mod's own requirement list hold? Returns a sentence when it doesn't.
    pub fn requirement_warning(&self) -> Option<String> {
        let c = self.config.dependencies.as_ref()?;
        if met(c, &HashMap::new(), &self.env) {
            return None;
        }
        let mut names = Vec::new();
        fn files(c: &Condition, out: &mut Vec<String>) {
            match c {
                Condition::File { file, state: FileState::Active } => out.push(file.clone()),
                Condition::All(v) | Condition::Any(v) => v.iter().for_each(|c| files(c, out)),
                _ => {}
            }
        }
        files(c, &mut names);
        Some(if names.is_empty() {
            "This mod says it needs something your game doesn't have.".into()
        } else {
            format!("This mod says it needs {}, which isn't turned on.", names.join(", "))
        })
    }

    /// The files to install with the choices made. Checks the current step first.
    pub fn finish(&self) -> Result<Finish, String> {
        self.check_current()?;
        let path = self.path();
        let flags = self.flags_of(&path);
        // (phase, priority, sequence, item)
        let mut items: Vec<(u8, i64, usize, InstallItem)> = Vec::new();
        let mut seq = 0usize;
        let mut push = |phase: u8, item: &InstallItem, items: &mut Vec<(u8, i64, usize, InstallItem)>| {
            items.push((phase, item.priority, seq, item.clone()));
            seq += 1;
        };
        for it in &self.config.required {
            push(0, it, &mut items);
        }
        for &si in &path {
            let before = {
                let pos = path.iter().position(|&x| x == si).unwrap_or(0);
                self.flags_of(&path[..pos])
            };
            for (gi, g) in self.config.steps[si].groups.iter().enumerate() {
                for (pi, p) in g.plugins.iter().enumerate() {
                    let chosen = self.sel[si][gi][pi];
                    let usable = p.kind.resolve(&before, &self.env) != PluginType::NotUsable;
                    for it in &p.files {
                        if chosen || it.always || (it.if_usable && usable) {
                            push(1, it, &mut items);
                        }
                    }
                }
            }
        }
        for (cond, list) in &self.config.conditional {
            if met(cond, &flags, &self.env) {
                for it in list {
                    push(2, it, &mut items);
                }
            }
        }
        items.sort_by_key(|(phase, prio, seq, _)| (*phase, *prio, *seq));

        let mut out: Vec<(String, String)> = Vec::new();
        let mut at: HashMap<String, usize> = HashMap::new();
        let mut missing = Vec::new();
        for (_, _, _, it) in &items {
            for (src, dest) in self.expand(it, &mut missing)? {
                let key = dest.to_lowercase();
                match at.get(&key) {
                    Some(&i) => out[i] = (src, dest),
                    None => {
                        at.insert(key, out.len());
                        out.push((src, dest));
                    }
                }
            }
        }
        let mut flags: Vec<(String, String)> = flags.into_iter().collect();
        flags.sort();
        Ok(Finish { files: out, missing, flags })
    }

    /// The archive path of a file the installer names (image or source), real casing, if it's there.
    pub fn archive_path(&self, installer_path: &str) -> Option<String> {
        let p = clean_path(installer_path)?;
        self.files
            .get(&join(&self.root, &p).to_lowercase())
            .or_else(|| self.files.get(&p.to_lowercase()))
            .cloned()
    }

    /// One `file` or `folder` entry as (archive path, destination) pairs.
    fn expand(&self, it: &InstallItem, missing: &mut Vec<String>) -> Result<Vec<(String, String)>, String> {
        let bad = |p: &str| format!("its installer points outside the mod ({p})");
        let src = clean_path(&it.source).ok_or_else(|| bad(&it.source))?;
        let dest = match &it.destination {
            None => src.clone(),
            Some(d) => clean_path(d).ok_or_else(|| bad(d))?,
        };
        let dest_is_folder = it.destination.as_deref().is_some_and(|d| d.trim().ends_with(['/', '\\']));
        if !it.folder {
            let Some(real) = self.archive_path(&src) else {
                missing.push(it.source.clone());
                return Ok(Vec::new());
            };
            let file_name = real.rsplit('/').next().unwrap_or(&real).to_string();
            let to = if dest.is_empty() || dest_is_folder { join(&dest, &file_name) } else { dest };
            return Ok(vec![(real, to)]);
        }
        // A folder: everything under it, keeping the layout below it. Tried under the installer's root first.
        let fomod_dir = join(&self.root, "fomod").to_lowercase() + "/";
        let mut found = Vec::new();
        for base in [join(&self.root, &src), src.clone()] {
            let prefix = if base.is_empty() { String::new() } else { format!("{}/", base.to_lowercase()) };
            for (lc, real) in &self.sorted {
                if lc.starts_with(&prefix) && !(src.is_empty() && lc.starts_with(&fomod_dir)) {
                    found.push((real.clone(), join(&dest, &real[prefix.len()..])));
                }
            }
            if !found.is_empty() || self.root.is_empty() {
                break;
            }
        }
        if found.is_empty() {
            missing.push(it.source.clone());
        }
        Ok(found)
    }

    /// The current step as JSON for the interface:
    /// {"module":..,"step":{"index","name","number","isLast","canGoBack","groups":[{"name","kind","hint","options":
    /// [{"name","description","image","type","selected","canChange"}]}]} or null,"warning":..}
    pub fn to_json(&self) -> String {
        let opt = |s: &Option<String>| s.as_deref().map(quote).unwrap_or_else(|| "null".into());
        let step = match self.current() {
            None => "null".to_string(),
            Some(v) => {
                let groups: Vec<String> = v
                    .groups
                    .iter()
                    .map(|g| {
                        let options: Vec<String> = g
                            .options
                            .iter()
                            .map(|o| {
                                format!(
                                    "{{\"name\":{},\"description\":{},\"image\":{},\"type\":{},\"selected\":{},\"canChange\":{}}}",
                                    quote(&o.name),
                                    quote(&o.description),
                                    opt(&o.image.as_deref().and_then(|i| self.archive_path(i))),
                                    quote(o.kind.key()),
                                    o.selected,
                                    o.can_change
                                )
                            })
                            .collect();
                        format!(
                            "{{\"name\":{},\"kind\":{},\"hint\":{},\"options\":[{}]}}",
                            quote(&g.name),
                            quote(g.kind.key()),
                            quote(g.kind.hint()),
                            options.join(",")
                        )
                    })
                    .collect();
                format!(
                    "{{\"index\":{},\"name\":{},\"number\":{},\"isLast\":{},\"canGoBack\":{},\"groups\":[{}]}}",
                    v.index,
                    quote(&v.name),
                    v.number,
                    v.is_last,
                    v.can_go_back,
                    groups.join(",")
                )
            }
        };
        format!(
            "{{\"module\":{},\"image\":{},\"step\":{},\"warning\":{}}}",
            quote(&self.config.module_name),
            opt(&self.config.module_image.as_deref().and_then(|i| self.archive_path(i))),
            step,
            opt(&self.requirement_warning())
        )
    }
}
