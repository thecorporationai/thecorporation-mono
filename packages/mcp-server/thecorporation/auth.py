"""MCP server authentication — auto-provisioning and shared config.

Resolution order:
1. CORP_API_KEY + CORP_WORKSPACE_ID env vars (explicit)
2. ~/.corp/config.json (shared with TUI/CLI)
3. Auto-provision via POST /v1/workspaces/provision
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass
class McpAuthContext:
    workspace_id: str
    api_key: str
    scopes: list[str] = field(default_factory=lambda: ["*"])


_CONFIG_FILE = Path.home() / ".corp" / "config.json"


def _load_config() -> dict[str, Any]:
    if _CONFIG_FILE.exists():
        with open(_CONFIG_FILE) as f:
            return json.load(f)
    return {}


def _save_config(cfg: dict[str, Any]) -> None:
    _CONFIG_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(_CONFIG_FILE, "w") as f:
        json.dump(cfg, f, indent=2)
        f.write("\n")


def _provision(api_url: str) -> dict[str, Any]:
    """Call the unauthenticated provision endpoint."""
    import httpx

    resp = httpx.post(
        f"{api_url.rstrip('/')}/v1/workspaces/provision",
        json={"name": "mcp-auto"},
        timeout=15.0,
    )
    resp.raise_for_status()
    return resp.json()


def resolve_or_provision_auth(
    api_url: str = "https://api.thecorporation.ai",
) -> McpAuthContext:
    """Resolve auth context from env, config file, or auto-provision.

    Args:
        api_url: Base URL of the backend API.

    Returns:
        McpAuthContext with workspace_id and api_key.
    """
    # 1. Env vars
    env_key = os.environ.get("CORP_API_KEY", "")
    env_ws = os.environ.get("CORP_WORKSPACE_ID", "")
    if env_key and env_ws:
        return McpAuthContext(workspace_id=env_ws, api_key=env_key)

    # 2. Config file
    cfg = _load_config()
    cfg_key = cfg.get("api_key", "")
    cfg_ws = cfg.get("workspace_id", "")
    if cfg_key and cfg_ws:
        return McpAuthContext(workspace_id=cfg_ws, api_key=cfg_key)

    # 3. Auto-provision
    result = _provision(api_url)
    # Save to config for next time (shared with TUI/CLI)
    cfg["api_key"] = result["api_key"]
    cfg["workspace_id"] = result["workspace_id"]
    if "api_url" not in cfg:
        cfg["api_url"] = api_url
    _save_config(cfg)

    return McpAuthContext(
        workspace_id=result["workspace_id"],
        api_key=result["api_key"],
    )
