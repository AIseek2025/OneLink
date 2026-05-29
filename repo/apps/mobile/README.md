# OneLink Mobile App

React Native + TypeScript mobile client for OneLink.

## Architecture

- **BFF Client**: `src/services/bffClient.ts` — unified client connecting to `/api/v1/bff/*`
  - Generic `request<T>()` helper with auth token injection, locale header, and structured error propagation
  - Exports `bffClient.get / .post / .patch / .del` for type-safe BFF calls
  - Auth token managed via `setAuthToken()` / `getAuthToken()`
  - `BffClientError` class carries `status`, `code`, `traceId` for traceable error handling
- **Config**: `src/services/config.ts` — BFF base URL, API prefix (`/api/v1/bff`), and async-storage-backed persistence for auth tokens, locale, region
- **Types**: `src/types/index.ts` — shared DTO types aligned with BFF contract freeze manifest
- **Theme**: `src/theme/tokens.ts` — design tokens (colors, spacing, radii, typography, component states) aligned with `repo/apps/web/src/design-tokens/tokens.json`

## Navigation

Built with `@react-navigation/native` v7:

- **Bottom Tabs**: `@react-navigation/bottom-tabs` v7 — primary navigation (首页/Lumi, 找人, 推荐, 消息, 我的)
- **Native Stack**: `@react-navigation/native-stack` v7 — screen-level navigation within each tab
- **Root**: `src/navigation/RootNavigator.tsx` — Splash → Auth (Login/Register) → MainTabs → Chat detail
- **App Entry**: `src/App.tsx` — SafeAreaProvider + RootNavigator

## Screens

| Screen | File | Tab | Key States |
|--------|------|-----|------------|
| Splash | `SplashScreen.tsx` | — | booting, restoring_session, session_restored, no_session, boot_failed |
| Login | `LoginScreen.tsx` | — | idle, submitting, login_success, login_failed, expired_session |
| Register | `RegisterScreen.tsx` | — | idle, validating, submitting, registered, requires_verification, failed |
| Home/Lumi | `HomeScreen.tsx` | 首页 | loading_list, empty_list, list_ready, load_failed |
| Chat | `ChatScreen.tsx` | — | ready, sending, reply_loading, reply_streaming, reply_completed, failed |
| Find | `FindScreen.tsx` | 找人 | draft, submitting, submitted, clarification_needed, failed |
| Recommendations | `RecommendationsScreen.tsx` | 推荐 | waiting_results, results_ready, empty_result, failed |
| Messages | `MessagesScreen.tsx` | 消息 | loading, empty, ready, failed |
| Me | `MeScreen.tsx` | 我的 | loading, ready, failed |

## BFF Boundary

All API calls go through the frozen BFF contract at `/api/v1/bff/*`. No direct backend or `/api/v1/app/*` paths are used. The BFF boundary was frozen in Phase 11 (Wave 0 Engineering Reset And Boundary Freeze); see `rules/17-BFF-CLIENT-CONTRACT-FREEZE.md` and `rules/26-WEB-APP-COMPLETION-PLAN.md`.

## Commands

```bash
npm start           # Start Metro bundler
npm run android     # Run on Android emulator
npm run ios         # Run on iOS simulator
npm run lint        # ESLint check
npm run typecheck   # TypeScript type check (tsc --noEmit)
npm run test        # Jest tests
```

## Environment Variables

| Variable | Dev Default | Production | Description |
|----------|-------------|------------|-------------|
| `BFF_BASE_URL` | `http://10.0.2.2:3000` | `https://api.onelink.app` | BFF service base URL |

- Android emulator uses `10.0.2.2` to reach host localhost
- iOS simulator can use `localhost` directly

## Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `react-native` | ^0.85.0 | Core runtime |
| `@react-navigation/native` | ^7.0.0 | Navigation framework |
| `@react-navigation/bottom-tabs` | ^7.0.0 | Tab navigation |
| `@react-navigation/native-stack` | ^7.0.0 | Stack navigation |
| `@react-native-async-storage/async-storage` | ^2.0.0 | Persistent token/locale storage |
| `react-native-safe-area-context` | ^5.0.0 | Safe area insets |
| `react-native-screens` | ^4.0.0 | Native screen optimization |
| `typescript` | ^5.7.0 | Type system |
| `jest` | ^29.0.0 | Test runner |
| `@testing-library/react-native` | ^12.0.0 | Component testing |
| `eslint` | ^9.0.0 | Linting |

## Status

Wave 0 Engineering Reset And Boundary Freeze (Phase 11). The mobile app has a complete navigation skeleton, BFF client, design tokens, auth flow, and all primary screen shells with empty/loading/error/degraded states. Full integration tests, real device builds, and remaining Wave 0 boundary cleanup are next.