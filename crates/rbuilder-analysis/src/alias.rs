//! May-alias heuristics for hybrid CPG P3 T2 (on-demand only).
//!
//! Conservative name-based rules — not a full points-to analysis:
//! - field access shares a base (`order` ↔ `order.status`)
//! - simple copy assignments `a = b` (identifier LHS/RHS) union names

use crate::cfg::ControlFlowGraph;
use std::collections::{HashMap, HashSet};

/// Expand `seed` to a may-alias name set for slicing / flows.
pub fn may_alias_names(cfg: &ControlFlowGraph, seed: &str) -> HashSet<String> {
    let mut parent: HashMap<String, String> = HashMap::new();

    fn ensure(parent: &mut HashMap<String, String>, name: &str) {
        parent
            .entry(name.to_string())
            .or_insert_with(|| name.to_string());
    }

    fn find(parent: &mut HashMap<String, String>, x: &str) -> String {
        let p = parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        if p == x {
            return p;
        }
        let root = find(parent, &p);
        parent.insert(x.to_string(), root.clone());
        root
    }

    fn union(parent: &mut HashMap<String, String>, a: &str, b: &str) {
        ensure(parent, a);
        ensure(parent, b);
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent.insert(ra, rb);
        }
    }

    ensure(&mut parent, seed);
    if let Some((base, _)) = seed.split_once('.') {
        union(&mut parent, seed, base);
    }

    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            for d in &stmt.defined_vars {
                ensure(&mut parent, d);
                if let Some((base, _)) = d.split_once('.') {
                    union(&mut parent, d, base);
                }
            }
            for u in &stmt.used_vars {
                ensure(&mut parent, u);
                if let Some((base, _)) = u.split_once('.') {
                    union(&mut parent, u, base);
                }
            }
            // Simple copy: exactly one defined local and one used local (no fields).
            if stmt.defined_vars.len() == 1 && stmt.used_vars.len() == 1 {
                let d = stmt.defined_vars.iter().next().unwrap();
                let u = stmt.used_vars.iter().next().unwrap();
                if !d.contains('.') && !u.contains('.') {
                    union(&mut parent, d, u);
                }
            }
        }
    }

    let seed_root = find(&mut parent, seed);
    let keys: Vec<String> = parent.keys().cloned().collect();
    keys.into_iter()
        .filter(|k| find(&mut parent, k) == seed_root)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_for_function;

    #[test]
    fn field_and_copy_alias() {
        let code = r#"
public class C {
    void m(OrderDTO order) {
        OrderDTO other = order;
        other.status = "X";
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "m").unwrap();
        let set = may_alias_names(&cfg, "order");
        assert!(set.contains("order"));
        assert!(set.contains("other"), "copy alias missing: set={set:?}");
        assert!(
            set.contains("other.status"),
            "field via alias missing: set={set:?}"
        );
    }
}
