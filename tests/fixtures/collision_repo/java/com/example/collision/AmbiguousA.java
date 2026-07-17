package com.example.collision;

/** Same bare name `collide` as {@link AmbiguousB} — ambiguous resolve bait. */
public class AmbiguousA {
    public static int collide() {
        return 1;
    }

    /** Polyglot short-name bait vs Rust `shared_leaf`. */
    public static int sharedLeaf() {
        return 10;
    }
}
