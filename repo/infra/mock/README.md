# OneLink API Mock Server

Frontend local development mock server using [Prism](https://stoplight.io/open-source/prism).

## Quick Start

```bash
cd repo/infra/mock
./start-mock.sh          # Starts on port 4010
./start-mock.sh 5000     # Custom port
```

## What's Mocked

All BFF aggregation endpoints and downstream service responses:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/bff/chat/init` | GET | Chat page init |
| `/api/v1/bff/onboarding` | GET | Onboarding flow |
| `/api/v1/bff/home` | GET | Home page |
| `/api/v1/bff/profile/{userId}` | GET | Profile view |
| `/api/v1/identity/register` | POST | Registration |
| `/api/v1/identity/login` | POST | Login |
| `/api/v1/identity/me` | GET | Current user |
| `/api/v1/chat/conversations` | POST | Create/get conversation |
| `/api/v1/chat/conversations/{id}/messages` | POST/GET | Send/list messages |
| `/api/v1/profile/me` | GET | Profile data |
| `/api/v1/profile/me/completion` | GET | Profile completion |
| `/api/v1/questions/pending` | GET | Pending questions |

## Authentication

The mock accepts any Bearer token — no validation is performed. Use any string:

```
Authorization: Bearer mock-dev-token
```

## Mock Data

Mock responses are defined in `mock-api.yaml` with realistic example data.
Edit the examples in the YAML to customize mock responses.

## Frontend Configuration

Point your frontend API base URL to the mock server:

```bash
# .env.local
VITE_API_BASE_URL=http://localhost:4010
```

## Running with Real Backend

For full-stack development, run the real services instead:

```bash
cd repo/scripts/local
./run-chat-memory-profile-slice.sh start-bg
```

Then point your frontend to the BFF:

```bash
VITE_API_BASE_URL=http://localhost:8083
```
