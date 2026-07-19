pub mod bytecode;
pub mod compiler;
pub mod files;
pub mod lex;
pub mod mir;
pub mod parse;
pub mod report;
pub mod runtime;


pub type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type HashSet<K> = rustc_hash::FxHashSet<K>;
// pub type HashMap<K, V> = std::collections::HashMap<K, V>;
// pub type HashSet<K> = std::collections::HashSet<K>;