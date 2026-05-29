#!/usr/bin/env bash
# OneLink API Mock Server — frontend local development
#
# Prerequisites: npx (Node.js 18+)
# Usage:
#   ./start-mock.sh          # Start mock on port 4010
#   ./start-mock.sh 5000     # Start mock on port 5000
#
# The mock server uses Prism (https://stoplight.io/open-source/prism)
# to serve realistic mock responses from mock-api.yaml.
# All BFF + downstream service endpoints are mocked.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOCK_SPEC="${SCRIPT_DIR}/mock-api.yaml"
PORT="${1:-4010}"

if [ ! -f "$MOCK_SPEC" ]; then
    echo "ERROR: mock spec not found at $MOCK_SPEC"
    exit 1
fi

echo "Starting OneLink API mock server on port ${PORT}..."
echo "Spec: ${MOCK_SPEC}"
echo "Endpoints:"
echo "  GET  http://localhost:${PORT}/api/v1/bff/chat/init"
echo "  GET  http://localhost:${PORT}/api/v1/bff/onboarding"
echo "  GET  http://localhost:${PORT}/api/v1/bff/home"
echo "  GET  http://localhost:${PORT}/api/v1/bff/profile/{userId}"
echo "  POST http://localhost:${PORT}/api/v1/identity/register"
echo "  POST http://localhost:${PORT}/api/v1/identity/login"
echo "  GET  http://localhost:${PORT}/api/v1/identity/me"
echo "  POST http://localhost:${PORT}/api/v1/chat/conversations"
echo "  POST http://localhost:${PORT}/api/v1/chat/conversations/{id}/messages"
echo "  GET  http://localhost:${PORT}/api/v1/profile/me"
echo "  GET  http://localhost:${PORT}/api/v1/profile/me/completion"
echo ""
echo "Note: Bearer token is accepted but not validated in mock mode."
echo "Press Ctrl+C to stop."

npx @stoplight/prism-cli mock --port "$PORT" "$MOCK_SPEC"
