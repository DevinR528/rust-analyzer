use std::{
    fmt,
    iter::FromIterator,
    sync::Arc,
};

use ra_syntax::{AstNode, TreeArc, SourceFile};
use ra_db::{
    SourceRootId,
    salsa::{Database, debug::{DebugQueryTable, TableEntry}},
};

use crate::{
    FileId, db::RootDatabase,
    symbol_index::{SymbolIndex, LibrarySymbolsQuery},
};

pub(crate) fn syntax_tree_stats(_db: &RootDatabase) -> SyntaxTreeStats {
    SyntaxTreeStats::default()
}

pub(crate) fn status(db: &RootDatabase) -> String {
    let files_stats = FilesStats::default();
    let syntax_tree_stats = syntax_tree_stats(db);
    let symbols_stats = db.query(LibrarySymbolsQuery).entries::<LibrarySymbolsStats>();
    format!(
        "{}\n{}\n{}\n\n\nmemory:\n{}\ngc {:?} seconds ago",
        files_stats,
        symbols_stats,
        syntax_tree_stats,
        MemoryStats::current(),
        db.last_gc.elapsed().as_secs(),
    )
}

#[derive(Default)]
struct FilesStats {
    total: usize,
    size: Bytes,
}

impl fmt::Display for FilesStats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} ({}) files", self.total, self.size)
    }
}

impl FromIterator<TableEntry<FileId, Arc<String>>> for FilesStats {
    fn from_iter<T>(iter: T) -> FilesStats
    where
        T: IntoIterator<Item = TableEntry<FileId, Arc<String>>>,
    {
        let mut res = FilesStats::default();
        for entry in iter {
            res.total += 1;
            res.size += entry.value.unwrap().len();
        }
        res
    }
}

#[derive(Default)]
pub(crate) struct SyntaxTreeStats {
    total: usize,
    pub(crate) retained: usize,
    retained_size: Bytes,
}

impl fmt::Display for SyntaxTreeStats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} trees, {} ({}) retained", self.total, self.retained, self.retained_size,)
    }
}

impl FromIterator<TableEntry<FileId, TreeArc<SourceFile>>> for SyntaxTreeStats {
    fn from_iter<T>(iter: T) -> SyntaxTreeStats
    where
        T: IntoIterator<Item = TableEntry<FileId, TreeArc<SourceFile>>>,
    {
        let mut res = SyntaxTreeStats::default();
        for entry in iter {
            res.total += 1;
            if let Some(value) = entry.value {
                res.retained += 1;
                res.retained_size += value.syntax().memory_size_of_subtree();
            }
        }
        res
    }
}

#[derive(Default)]
struct LibrarySymbolsStats {
    total: usize,
    size: Bytes,
}

impl fmt::Display for LibrarySymbolsStats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} ({}) symbols", self.total, self.size,)
    }
}

impl FromIterator<TableEntry<SourceRootId, Arc<SymbolIndex>>> for LibrarySymbolsStats {
    fn from_iter<T>(iter: T) -> LibrarySymbolsStats
    where
        T: IntoIterator<Item = TableEntry<SourceRootId, Arc<SymbolIndex>>>,
    {
        let mut res = LibrarySymbolsStats::default();
        for entry in iter {
            let value = entry.value.unwrap();
            res.total += value.len();
            res.size += value.memory_size();
        }
        res
    }
}

struct MemoryStats {
    allocated: Bytes,
    resident: Bytes,
}

impl MemoryStats {
    #[cfg(feature = "jemalloc")]
    fn current() -> MemoryStats {
        jemalloc_ctl::epoch().unwrap();
        MemoryStats {
            allocated: Bytes(jemalloc_ctl::stats::allocated().unwrap()),
            resident: Bytes(jemalloc_ctl::stats::resident().unwrap()),
        }
    }

    #[cfg(not(feature = "jemalloc"))]
    fn current() -> MemoryStats {
        MemoryStats { allocated: Bytes(0), resident: Bytes(0) }
    }
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{} allocated {} resident", self.allocated, self.resident,)
    }
}

#[derive(Default)]
struct Bytes(usize);

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bytes = self.0;
        if bytes < 4096 {
            return write!(f, "{} bytes", bytes);
        }
        let kb = bytes / 1024;
        if kb < 4096 {
            return write!(f, "{}kb", kb);
        }
        let mb = kb / 1024;
        write!(f, "{}mb", mb)
    }
}

impl std::ops::AddAssign<usize> for Bytes {
    fn add_assign(&mut self, x: usize) {
        self.0 += x;
    }
}
