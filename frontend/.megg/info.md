---
created: 2025-12-15T09:25:00.584Z
updated: 2025-12-16T12:03:02.361Z
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

## 2025-12-15T10:20:12.946Z

## Key Implementation Notes

### Routing
- `/` - Dashboard (system stats + quick actions)
- `/system-monitor` - Detailed system monitoring with charts
- `/scripts` - Script execution with terminal output
- `/history` - Task execution history
- `/settings` - Scripts directory + quick action management

### Quick Actions Flow
Dashboard -> click Quick Action -> navigates to `/scripts?script=<name>` -> Scripts page auto-selects script

### Type Imports
Project uses `verbatimModuleSyntax` in TypeScript. Always use:
```typescript
import { api } from '../lib/api';
import type { QuickAction } from '../lib/api';
```
Not:
```typescript
import { api, QuickAction } from '../lib/api'; // Error!
```


## 2025-12-16T12:03:02.362Z
## Key Implementation Notes

### Routing
- `/` - Dashboard (system stats + quick actions)
- `/system-monitor` - Detailed system monitoring with charts
- `/scripts` - Script execution with terminal output (Admin Only)
- `/history` - Task execution history
- `/settings` - Scripts directory + quick action management (Admin Only)

### Quick Actions Flow
Dashboard -> click Quick Action -> **POST /api/quick-actions/:id/execute** -> navigates to `/history?highlight_task=<id>` (accessible to Clients)

### Type Imports
Project uses `verbatimModuleSyntax` in TypeScript. Always use:
```typescript
import { api } from '../lib/api';
import type { QuickAction } from '../lib/api';
```