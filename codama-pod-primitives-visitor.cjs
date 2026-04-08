"use strict";

/**
 * Maps spl_pod-style IDL aliases (PodU64 / PodU32) to plain number types after
 * Anchor → Codama conversion (defined type links become podU64 / podU32).
 */
const { bottomUpTransformerVisitor, numberTypeNode, assertIsNode } = require("codama");

const POD_TO_PRIMITIVE = {
  podU64: () => numberTypeNode("u64"),
  podU32: () => numberTypeNode("u32"),
};

module.exports = bottomUpTransformerVisitor([
  {
    select: "[definedTypeLinkNode]",
    transform: (node) => {
      assertIsNode(node, "definedTypeLinkNode");
      const build = POD_TO_PRIMITIVE[node.name];
      return build ? build() : node;
    },
  },
]);
