use std::sync::Arc;
use rustc_hash::FxHashMap;

use ra_syntax::{SmolStr, ast};

use crate::{
    Crate, DefDatabase, Enum, Function, HirDatabase, ImplBlock, Module, Static, Struct, Trait, AstDatabase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangItemTarget {
    Enum(Enum),
    Function(Function),
    ImplBlock(ImplBlock),
    Static(Static),
    Struct(Struct),
    Trait(Trait),
}

impl LangItemTarget {
    pub(crate) fn krate(&self, db: &impl HirDatabase) -> Option<Crate> {
        match self {
            LangItemTarget::Enum(e) => e.module(db),
            LangItemTarget::Function(f) => f.module(db),
            LangItemTarget::ImplBlock(i) => i.module(db),
            LangItemTarget::Static(s) => s.module(db),
            LangItemTarget::Struct(s) => s.module(db),
            LangItemTarget::Trait(t) => t.module(db),
        }
        .krate(db)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangItems {
    items: FxHashMap<SmolStr, LangItemTarget>,
}

impl LangItems {
    pub fn target<'a>(&'a self, item: &str) -> Option<&'a LangItemTarget> {
        self.items.get(item)
    }

    /// Salsa query. This will look for lang items in a specific crate.
    pub(crate) fn lang_items_query(
        db: &(impl DefDatabase + AstDatabase),
        krate: Crate,
    ) -> Arc<LangItems> {
        let mut lang_items = LangItems { items: FxHashMap::default() };

        if let Some(module) = krate.root_module(db) {
            lang_items.collect_lang_items_recursive(db, &module);
        }

        Arc::new(lang_items)
    }

    /// Salsa query. Look for a lang item, starting from the specified crate and recursively
    /// traversing its dependencies.
    pub(crate) fn lang_item_query(
        db: &impl DefDatabase,
        start_crate: Crate,
        item: SmolStr,
    ) -> Option<LangItemTarget> {
        let lang_items = db.lang_items(start_crate);
        let start_crate_target = lang_items.items.get(&item);
        if let Some(target) = start_crate_target {
            Some(*target)
        } else {
            for dep in start_crate.dependencies(db) {
                let dep_crate = dep.krate;
                let dep_target = db.lang_item(dep_crate, item.clone());
                if dep_target.is_some() {
                    return dep_target;
                }
            }
            None
        }
    }

    fn collect_lang_items_recursive(
        &mut self,
        db: &(impl DefDatabase + AstDatabase),
        module: &Module,
    ) {
        // Look for impl targets
        for impl_block in module.impl_blocks(db) {
            if let Some(lang_item_name) = impl_block.lang_item(db) {
                self.items.entry(lang_item_name).or_insert(LangItemTarget::ImplBlock(impl_block));
            }
        }

        // FIXME we should look for the other lang item targets (traits, structs, ...)

        // Look for lang items in the children
        for child in module.children(db) {
            self.collect_lang_items_recursive(db, &child);
        }
    }
}

pub(crate) fn lang_item_from_ast(node: &impl ast::AttrsOwner) -> Option<SmolStr> {
    node.attrs()
        .filter_map(|a| a.as_key_value())
        .filter(|(key, _)| key == "lang")
        .map(|(_, val)| val)
        .nth(0)
}
