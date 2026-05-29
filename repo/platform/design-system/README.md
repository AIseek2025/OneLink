# OneLink Design System — Phase 2 Baseline

## Overview

This directory contains the design system tokens and component specifications for OneLink App/Web. Tokens are the single source of truth for visual properties; components reference tokens, never hard-code values.

## Token Source

- `tokens.json` — Design Tokens Community Group format, consumable by Style Dictionary / token-transformers

## Component Naming Convention

- `Ol` prefix for all components (e.g., `OlButton`, `OlChatBubble`, `OlProfileCard`)
- PascalCase for component names
- kebab-case for CSS class names (e.g., `ol-button`, `ol-chat-bubble`)

## States

Every interactive component must define:

| State | Token Reference |
|-------|----------------|
| Default | `color.brand.primary` |
| Hover | `color.brand.primary-hover` |
| Focus | `color.neutral.border-focus` + ring |
| Disabled | `color.neutral.text-placeholder` + opacity 0.5 |
| Loading | Spinner overlay + `color.brand.primary` |
| Error | `color.semantic.error` |

## Empty States

| Context | Display |
|---------|---------|
| No chat messages | "Start a conversation with Lumi" + chat icon |
| Profile incomplete | Completion card with missing dimensions |
| No search results | "No matches yet — try adjusting your criteria" |
| DM list empty | "No conversations yet" |

## Error States

| Error Type | Display |
|------------|---------|
| Network error | Toast: "Connection lost. Retrying…" + retry button |
| Auth expired | Redirect to login with toast: "Session expired" |
| 403 Forbidden | "You don't have permission to view this" |
| 502 Upstream | "Service temporarily unavailable. Please try again." |
| Generic | Toast with error message from response |

## Iconography

- Icon set: Lucide Icons (MIT license, tree-shakeable)
- Size tokens: `sm=16px`, `md=20px`, `lg=24px`
- Stroke width: 1.5px default

## Motion

- Transition duration: `fast=150ms`, `normal=250ms`, `slow=350ms`
- Easing: `ease-in-out` default
- No motion for `prefers-reduced-motion: reduce`
