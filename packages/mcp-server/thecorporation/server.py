"""The Corporation — MCP server for agent-native corporate operations.

Uses the shared corp_tools package for tool definitions and implementations.
Auto-registers all tools from the shared registry at import time.
"""

from __future__ import annotations

import json

from mcp.server.fastmcp import FastMCP

from corp_tools.definitions import TOOL_REGISTRY
from corp_tools.engine import dispatch, new_empty_stores, STORE_NAMES

mcp = FastMCP("thecorporation")

# ---------------------------------------------------------------------------
# Workspace context — set once at startup via auth resolution
# ---------------------------------------------------------------------------

_workspace_id: str | None = None


def set_workspace_context(ws_id: str) -> None:
    global _workspace_id
    _workspace_id = ws_id


def get_workspace_id() -> str:
    if _workspace_id is None:
        return "default"
    return _workspace_id


# ---------------------------------------------------------------------------
# In-memory state — scoped by workspace_id
# Each store is {workspace_id: {item_id: item_dict}}
# ---------------------------------------------------------------------------

_stores: dict[str, dict[str, dict[str, dict]]] = {name: {} for name in STORE_NAMES}


def reset_mcp_stores() -> None:
    """Clear all in-memory stores (for tests)."""
    global _workspace_id
    _workspace_id = None
    for store in _stores.values():
        store.clear()


def _ws_stores() -> dict[str, dict]:
    """Get the flat stores dict scoped to the current workspace."""
    ws = get_workspace_id()
    result = {}
    for name in STORE_NAMES:
        if ws not in _stores[name]:
            _stores[name][ws] = {}
        result[name] = _stores[name][ws]
    return result


# ---------------------------------------------------------------------------
# Auto-register tools from the shared registry
# ---------------------------------------------------------------------------

def _make_tool_fn(tool_name: str):
    """Create a tool function that dispatches to the shared engine."""
    meta = TOOL_REGISTRY[tool_name]
    params = meta.get("parameters", {})
    props = params.get("properties", {})
    required = set(params.get("required", []))

    # Build docstring from schema for MCP introspection
    doc_lines = [meta.get("description", "")]
    if props:
        doc_lines.append("")
        doc_lines.append("Args:")
        for pname, pinfo in props.items():
            desc = pinfo.get("description", "")
            doc_lines.append(f"    {pname}: {desc}")

    def tool_fn(**kwargs) -> str:
        stores = _ws_stores()
        return dispatch(tool_name, kwargs, stores)

    tool_fn.__name__ = tool_name
    tool_fn.__doc__ = "\n".join(doc_lines)

    # Build type annotations so FastMCP can introspect parameters
    type_map = {"string": str, "integer": int, "boolean": bool, "number": float}
    annotations = {}
    defaults = {}
    for pname, pinfo in props.items():
        ptype = pinfo.get("type", "string")
        if ptype == "array":
            annotations[pname] = list
        elif ptype == "object":
            annotations[pname] = dict
        else:
            annotations[pname] = type_map.get(ptype, str)
        if pname not in required:
            defaults[pname] = "" if ptype == "string" else None
    annotations["return"] = str
    tool_fn.__annotations__ = annotations
    tool_fn.__defaults__ = tuple(defaults.values()) if defaults else None

    return tool_fn


for _tool_name in TOOL_REGISTRY:
    _fn = _make_tool_fn(_tool_name)
    mcp.tool()(_fn)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main():
    import os
    from thecorporation.auth import resolve_or_provision_auth

    api_url = os.environ.get("CORP_API_URL", "https://api.thecorporation.ai")
    try:
        ctx = resolve_or_provision_auth(api_url=api_url)
        set_workspace_context(ctx.workspace_id)
    except Exception:
        pass  # Offline mode — workspace_id stays None, uses "default"

    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
