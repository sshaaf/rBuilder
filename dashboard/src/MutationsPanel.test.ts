import { describe, expect, it } from "vitest";
import { filterMutationWrites } from "./MutationsPanel";
import type { MutationWriteEntry } from "./types";

const sample: MutationWriteEntry[] = [
  {
    function_id: "a",
    function_name: "setCartTotal",
    is_constructor: false,
    receiver_type: "ShoppingCart",
    member: "cartTotal",
    file: "ShoppingCart.java",
    line: 61,
    code_snippet: "this.cartTotal = cartTotal",
    kind: "ThisField",
  },
  {
    function_id: "b",
    function_name: "ShoppingCart",
    is_constructor: true,
    receiver_type: "ShoppingCart",
    member: "cartId",
    file: "ShoppingCart.java",
    line: 10,
    code_snippet: "this.cartId = cartId",
    kind: "ThisField",
  },
  {
    function_id: "c",
    function_name: "mystery",
    is_constructor: false,
    member: "x",
    file: "Other.java",
    line: 3,
    code_snippet: "obj.x = 1",
    kind: "Unresolved",
  },
];

describe("filterMutationWrites", () => {
  it("filters by type and excludes constructors by default pattern", () => {
    const hits = filterMutationWrites(sample, "ShoppingCart", true, "", false);
    expect(hits).toHaveLength(1);
    expect(hits[0].member).toBe("cartTotal");
  });

  it("matches FQN suffix types", () => {
    const withFqn: MutationWriteEntry[] = [
      {
        ...sample[0],
        receiver_type: "com.example.ecommerce.coolstore.model.ShoppingCart",
      },
    ];
    const hits = filterMutationWrites(withFqn, "ShoppingCart", true, "", false);
    expect(hits).toHaveLength(1);
  });

  it("includes unresolved only when requested and type empty", () => {
    expect(filterMutationWrites(sample, "", true, "", true)).toHaveLength(2);
    expect(filterMutationWrites(sample, "ShoppingCart", true, "", true)).toHaveLength(1);
  });
});
