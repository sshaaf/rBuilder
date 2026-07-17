//! Cross-file short-name collision bait (`twin` in two modules).

pub mod twin_a;
pub mod twin_b;

/// Polyglot short-name bait vs Java `sharedLeaf`.
pub fn shared_leaf() -> i32 {
    20
}
