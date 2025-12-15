---
created: 2025-12-15T09:25:00.584Z
updated: 2025-12-15T09:25:00.584Z
type: context
---
# Frontend Context

## Stack
- React 19 + TypeScript
- Vite for bundling (builds to dist/)
- Tailwind CSS + shadcn/ui components
- React Router for navigation
- Recharts for data visualization

## Structure
- `/pages` - Dashboard, Scripts, Settings, SystemMonitor
- `/components/ui` - shadcn/ui primitives (do not edit directly)
- `/components` - App-level components (Layout)
- `/hooks` - useWebSocket, useSystemStats
- `/lib` - API client, utilities

## Conventions
- Use shadcn/ui components from /components/ui
- Custom hooks for WebSocket and data fetching
- API calls go through /lib/api.ts
- Mobile-first responsive design

## Branding Integration
Apply ToruAI colors:
- Primary buttons/accents: #493FAA
- Hover states: #7E64E7  
- Background: #F9F9F9
- Text: #191919 (primary), #494949 (secondary)

## API Endpoints
- GET /api/health - Health check
- GET /api/resources - CPU, RAM, uptime
- GET /api/scripts - List scripts
- GET/PUT /api/settings - Configuration
- GET /api/history - Task history
- GET/POST/DELETE /api/quick-actions - Quick actions
- WS /api/ws - Real-time terminal