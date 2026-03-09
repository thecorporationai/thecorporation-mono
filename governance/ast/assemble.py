#!/usr/bin/env python3
"""Assemble split AST files into governance-ast.json.

Run from repo root or pass --ast-dir to specify the AST directory.
This script is called by build.rs before cargo compilation.
"""
import json
import os
import sys

def assemble(ast_dir):
    # Load meta
    with open(os.path.join(ast_dir, "meta.json")) as f:
        ast = json.load(f)

    # Load rules
    rules_path = os.path.join(ast_dir, "rules.json")
    if os.path.exists(rules_path):
        with open(rules_path) as f:
            ast["rules"] = json.load(f)

    # Load structured data
    sd_path = os.path.join(ast_dir, "structured-data.json")
    if os.path.exists(sd_path):
        with open(sd_path) as f:
            ast["structured_data"] = json.load(f)

    # Load documents in manifest order
    docs_dir = os.path.join(ast_dir, "documents")
    with open(os.path.join(docs_dir, "_manifest.json")) as f:
        manifest = json.load(f)

    ast["documents"] = []
    for doc_id in manifest:
        filename = doc_id.replace("_", "-") + ".json"
        path = os.path.join(docs_dir, filename)
        with open(path) as f:
            ast["documents"].append(json.load(f))

    # Write assembled file
    out_path = os.path.join(ast_dir, "governance-ast.json")
    with open(out_path, "w") as f:
        json.dump(ast, f, indent=2, ensure_ascii=False)
        f.write("\n")

    return len(ast["documents"])

if __name__ == "__main__":
    ast_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__))
    count = assemble(ast_dir)
    print(f"Assembled {count} documents into governance-ast.json")
