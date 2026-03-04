#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT="corp-integration"

export API_PORT="${API_PORT:-8000}"
export CORP_API_URL="http://localhost:${API_PORT}"

cd "$SCRIPT_DIR"

up() {
  echo "▸ Building and starting api-rs…"
  docker compose -p "$PROJECT" up -d --build --wait
  echo "✓ API ready at $CORP_API_URL"
}

down() {
  echo "▸ Tearing down…"
  docker compose -p "$PROJECT" down -v --remove-orphans 2>/dev/null || true
}

test_ts() {
  echo "▸ Running TypeScript integration tests…"
  cd "$SCRIPT_DIR/../../packages/corp-tools"
  CORP_API_URL="$CORP_API_URL" npx vitest run --config vitest.integration.config.ts
}

case "${1:-help}" in
  up)   up ;;
  down) down ;;
  test)
    trap down EXIT
    up
    test_ts
    echo "✓ All integration tests passed"
    ;;
  *)
    echo "Usage: $0 {up|down|test}"
    exit 1
    ;;
esac
