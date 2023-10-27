use std::path::Path;

use indexmap::IndexMap;
use kclvm_error::Position;

use crate::core::symbol::SymbolRef;

use super::{package::ModuleInfo, symbol::KCLSymbolData};

pub trait Scope {
    type SymbolData;
    fn get_filename(&self) -> &str;
    fn get_parent(&self) -> Option<ScopeRef>;
    fn get_children(&self) -> Vec<ScopeRef>;

    fn contains_pos(&self, pos: &Position) -> bool;

    fn get_owner(&self) -> Option<SymbolRef>;
    fn get_defs(&self) -> Option<&IndexMap<String, SymbolRef>>;
    fn get_refs(&self) -> &Vec<SymbolRef>;
    fn look_up_def(
        &self,
        name: &str,
        scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Option<SymbolRef>;

    fn get_all_defs(
        &self,
        scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Vec<SymbolRef>;

    fn dump(&self, scope_data: &ScopeData, symbol_data: &Self::SymbolData) -> Option<String>;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum ScopeKind {
    Local,
    Root,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ScopeRef {
    pub(crate) id: generational_arena::Index,
    pub(crate) kind: ScopeKind,
}

impl ScopeRef {
    pub fn get_id(&self) -> generational_arena::Index {
        self.id
    }

    pub fn get_kind(&self) -> ScopeKind {
        self.kind
    }
}

#[derive(Default, Debug)]
pub struct ScopeData {
    /// map pkgpath to root_scope
    pub(crate) root_map: IndexMap<String, ScopeRef>,
    pub(crate) locals: generational_arena::Arena<LocalSymbolScope>,
    pub(crate) roots: generational_arena::Arena<RootSymbolScope>,
}

impl ScopeData {
    pub fn get_scope(&self, scope: ScopeRef) -> Option<&dyn Scope<SymbolData = KCLSymbolData>> {
        match scope.get_kind() {
            ScopeKind::Local => {
                Some(self.locals.get(scope.get_id())? as &dyn Scope<SymbolData = KCLSymbolData>)
            }
            ScopeKind::Root => {
                Some(self.roots.get(scope.get_id())? as &dyn Scope<SymbolData = KCLSymbolData>)
            }
        }
    }

    pub fn add_def_to_scope(&mut self, scope: ScopeRef, name: String, symbol: SymbolRef) {
        match scope.get_kind() {
            ScopeKind::Local => {
                if let Some(local) = self.locals.get_mut(scope.get_id()) {
                    local.defs.insert(name, symbol);
                }
            }
            ScopeKind::Root => {
                unreachable!("never add symbol to root scope after namer pass")
            }
        }
    }

    pub fn add_ref_to_scope(&mut self, scope: ScopeRef, symbol: SymbolRef) {
        match scope.get_kind() {
            ScopeKind::Local => {
                if let Some(local) = self.locals.get_mut(scope.get_id()) {
                    local.refs.push(symbol);
                }
            }
            ScopeKind::Root => {
                if let Some(root) = self.roots.get_mut(scope.get_id()) {
                    root.refs.push(symbol);
                }
            }
        }
    }

    pub fn set_owner_to_scope(&mut self, scope: ScopeRef, owner: SymbolRef) {
        match scope.get_kind() {
            ScopeKind::Local => {
                if let Some(local) = self.locals.get_mut(scope.get_id()) {
                    local.owner = Some(owner);
                }
            }
            ScopeKind::Root => {
                if let Some(root) = self.roots.get_mut(scope.get_id()) {
                    root.owner = owner;
                }
            }
        }
    }

    pub fn alloc_root_scope(&mut self, root: RootSymbolScope) -> ScopeRef {
        let filepath = root.pkgpath.clone();
        let id = self.roots.insert(root);
        let scope_ref = ScopeRef {
            id,
            kind: ScopeKind::Root,
        };
        self.root_map.insert(filepath, scope_ref);
        scope_ref
    }

    pub fn alloc_local_scope(&mut self, local: LocalSymbolScope) -> ScopeRef {
        let id = self.locals.insert(local);
        ScopeRef {
            id,
            kind: ScopeKind::Local,
        }
    }
}

#[derive(Debug)]
pub struct RootSymbolScope {
    pub(crate) pkgpath: String,

    pub(crate) filename: String,

    /// PackageSymbol of this scope
    pub(crate) owner: SymbolRef,

    /// map filepath to children
    pub(crate) children: IndexMap<String, Vec<ScopeRef>>,

    pub(crate) refs: Vec<SymbolRef>,
}

impl Scope for RootSymbolScope {
    type SymbolData = KCLSymbolData;
    fn get_filename(&self) -> &str {
        &self.filename
    }

    fn get_children(&self) -> Vec<ScopeRef> {
        let mut children = vec![];
        for scopes in self.children.values() {
            children.append(&mut scopes.clone())
        }
        children
    }

    fn get_parent(&self) -> Option<ScopeRef> {
        None
    }

    fn contains_pos(&self, pos: &Position) -> bool {
        let real_pkg_path = if self.filename.ends_with(".k") {
            Path::new(self.filename.strip_suffix(".k").unwrap())
        } else {
            Path::new(&self.filename)
        };
        let real_pos_path = if pos.filename.ends_with(".k") {
            Path::new(pos.filename.strip_suffix(".k").unwrap())
        } else {
            Path::new(&pos.filename)
        };
        if real_pkg_path != real_pos_path {
            if let Some(parent) = real_pos_path.parent() {
                real_pkg_path == parent
            } else {
                false
            }
        } else {
            true
        }
    }
    fn get_owner(&self) -> Option<SymbolRef> {
        Some(self.owner)
    }

    fn get_defs(&self) -> Option<&IndexMap<String, SymbolRef>> {
        None
    }

    fn get_refs(&self) -> &Vec<SymbolRef> {
        &self.refs
    }

    fn look_up_def(
        &self,
        name: &str,
        _scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Option<SymbolRef> {
        let package_symbol = symbol_data.get_symbol(self.owner)?;

        package_symbol.get_attribute(name, symbol_data, module_info)
    }

    fn get_all_defs(
        &self,
        _scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Vec<SymbolRef> {
        if let Some(owner) = symbol_data.get_symbol(self.owner) {
            owner.get_all_attributes(symbol_data, module_info)
        } else {
            vec![]
        }
    }

    fn dump(&self, scope_data: &ScopeData, symbol_data: &Self::SymbolData) -> Option<String> {
        let mut output = String::from("");
        output.push_str("{\n\"scope_kind\": \"Root\",\n");
        output.push_str(&format!("\n\"pkgpath\": \"{}\",\n", self.pkgpath));
        let owner_symbol = symbol_data.get_symbol(self.owner)?;
        output.push_str(&format!(
            "\"owner\": {},\n",
            owner_symbol.full_dump(symbol_data)?
        ));
        output.push_str("\"refs\": [\n");
        for (index, symbol) in self.refs.iter().enumerate() {
            let symbol = symbol_data.get_symbol(*symbol)?;
            output.push_str(&format!("{}", symbol.full_dump(symbol_data)?));
            if index + 1 < self.refs.len() {
                output.push_str(",\n")
            }
        }
        output.push_str("\n],\n");
        output.push_str("\"children\": {\n");
        for (index, (key, scopes)) in self.children.iter().enumerate() {
            output.push_str(&format!("\"{}\": [\n", key));
            for (index, scope) in scopes.iter().enumerate() {
                let scope = scope_data.get_scope(*scope)?;
                output.push_str(&format!("{}", scope.dump(scope_data, symbol_data)?));
                if index + 1 < self.children.len() {
                    output.push_str(",\n");
                }
            }
            output.push_str("\n]");
            if index + 1 < self.children.len() {
                output.push_str(",\n");
            }
        }
        output.push_str("\n}\n}");

        let val: serde_json::Value = serde_json::from_str(&output).unwrap();
        Some(serde_json::to_string_pretty(&val).ok()?)
    }
}

impl RootSymbolScope {
    pub fn new(pkgpath: String, filename: String, owner: SymbolRef) -> Self {
        Self {
            pkgpath,
            filename,
            owner,
            children: IndexMap::default(),
            refs: vec![],
        }
    }

    pub fn add_child(&mut self, filepath: &str, child: ScopeRef) {
        if self.children.contains_key(filepath) {
            self.children.get_mut(filepath).unwrap().push(child);
        } else {
            self.children.insert(filepath.to_string(), vec![child]);
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct LocalSymbolScope {
    pub(crate) parent: ScopeRef,
    pub(crate) owner: Option<SymbolRef>,
    pub(crate) children: Vec<ScopeRef>,
    pub(crate) defs: IndexMap<String, SymbolRef>,
    pub(crate) refs: Vec<SymbolRef>,

    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Scope for LocalSymbolScope {
    type SymbolData = KCLSymbolData;

    fn get_filename(&self) -> &str {
        &self.start.filename
    }

    fn get_children(&self) -> Vec<ScopeRef> {
        self.children.clone()
    }

    fn get_parent(&self) -> Option<ScopeRef> {
        Some(self.parent)
    }

    fn contains_pos(&self, pos: &Position) -> bool {
        self.start.filename == pos.filename
            && self.start.less_equal(pos)
            && pos.less_equal(&self.end)
    }

    fn get_owner(&self) -> Option<SymbolRef> {
        self.owner.clone()
    }

    fn get_defs(&self) -> Option<&IndexMap<String, SymbolRef>> {
        Some(&self.defs)
    }

    fn get_refs(&self) -> &Vec<SymbolRef> {
        &self.refs
    }

    fn look_up_def(
        &self,
        name: &str,
        scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Option<SymbolRef> {
        match self.defs.get(name) {
            Some(symbol_ref) => return Some(*symbol_ref),
            None => {
                if let Some(owner) = self.owner.as_ref() {
                    let owner_symbol = symbol_data.get_symbol(*owner)?;
                    if let Some(symbol_ref) =
                        owner_symbol.get_attribute(name, symbol_data, module_info)
                    {
                        return Some(symbol_ref);
                    }
                };
                let parent = scope_data.get_scope(self.parent)?;
                parent.look_up_def(name, scope_data, symbol_data, module_info)
            }
        }
    }

    fn get_all_defs(
        &self,
        scope_data: &ScopeData,
        symbol_data: &Self::SymbolData,
        module_info: Option<&ModuleInfo>,
    ) -> Vec<SymbolRef> {
        let mut result = vec![];
        for def in self.defs.values() {
            result.push(*def);
        }
        if let Some(owner) = self.owner {
            if let Some(owner) = symbol_data.get_symbol(owner) {
                result.append(&mut owner.get_all_attributes(symbol_data, module_info));
            }
        }
        if let Some(parent) = scope_data.get_scope(self.parent) {
            result.append(&mut parent.get_all_defs(scope_data, symbol_data, module_info));
        }
        result.sort();
        result
    }

    fn dump(&self, scope_data: &ScopeData, symbol_data: &Self::SymbolData) -> Option<String> {
        let mut output = String::from("");
        output.push_str("{\n\"scope_kind\": \"Local\",\n");
        output.push_str(&format!(
            "\"range\": \"{}:{}",
            self.start.filename, self.start.line
        ));
        if let Some(start_col) = self.start.column {
            output.push_str(&format!(":{}", start_col));
        }

        output.push_str(&format!(" to {}", self.end.line));
        if let Some(end_col) = self.end.column {
            output.push_str(&format!(":{}", end_col));
        }
        output.push_str("\",\n");
        if let Some(owner) = self.owner.as_ref() {
            let owner_symbol = symbol_data.get_symbol(*owner)?;
            output.push_str(&format!(
                "\"owner\": {},\n",
                owner_symbol.full_dump(symbol_data)?
            ));
        }
        output.push_str("\"defs\": {\n");
        for (index, (key, symbol)) in self.defs.iter().enumerate() {
            let symbol = symbol_data.get_symbol(*symbol)?;
            output.push_str(&format!("\"{}\": {}", key, symbol.full_dump(symbol_data)?));
            if index + 1 < self.defs.len() {
                output.push_str(",\n")
            }
        }
        output.push_str("\n},\n");
        output.push_str("\"refs\": [\n");
        for (index, symbol) in self.refs.iter().enumerate() {
            let symbol = symbol_data.get_symbol(*symbol)?;
            output.push_str(&format!("{}", symbol.full_dump(symbol_data)?));
            if index + 1 < self.refs.len() {
                output.push_str(",\n")
            }
        }
        output.push_str("\n],");
        output.push_str("\n\"children\": [\n");
        for (index, scope) in self.children.iter().enumerate() {
            let scope = scope_data.get_scope(*scope)?;
            output.push_str(&format!("{}", scope.dump(scope_data, symbol_data)?));
            if index + 1 < self.children.len() {
                output.push_str(",\n")
            }
        }
        output.push_str("\n]\n}");
        Some(output)
    }
}

impl LocalSymbolScope {
    pub fn new(parent: ScopeRef, start: Position, end: Position) -> Self {
        Self {
            parent,
            owner: None,
            children: vec![],
            defs: IndexMap::default(),
            refs: vec![],
            start,
            end,
        }
    }

    pub fn add_child(&mut self, child: ScopeRef) {
        self.children.push(child)
    }

    pub fn set_owner(&mut self, owner: SymbolRef) {
        self.owner = Some(owner)
    }
}
