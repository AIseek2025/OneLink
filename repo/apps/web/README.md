# OneLink Web Client

React + Vite + TypeScript web client for OneLink.

## Architecture

- All API calls go through the frozen BFF contract at `/api/v1/bff/*`
- No direct backend or `/api/v1/app/*` paths are used
- OpenAPI mock available via `npm run mock`

## Commands

```bash
npm run dev          # Start Vite dev server
npm run build        # TypeScript compile + Vite production build
npm run lint         # ESLint check
npm run typecheck    # TypeScript type check (noEmit)
npm run test         # Vitest unit tests
npm run mock         # Start Prism mock server from OpenAPI spec
npm run mock:verify  # Verify OpenAPI mock compatibility
```

## BFF Boundary

All API calls go through `/api/v1/bff/*` exclusively. The `/api/v1/app/*` route has been removed as part of Wave 0 boundary freeze.

## Status

Wave 0 (Phase 11): engineering reset and boundary freeze. Web client connects exclusively through the frozen BFF contract.
