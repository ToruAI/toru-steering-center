---
created: 2025-12-15T09:25:00.584Z
updated: 2025-12-30T12:45:00.000Z
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
- `/pages` - Dashboard, Scripts, Settings, SystemMonitor, **Plugins** (planned)
- `/components/ui` - shadcn/ui primitives (do not edit directly)
- `/components` - App-level components (Layout, **PluginView** planned)
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

## Routing
- `/` - Dashboard (system stats + quick actions)
- `/system-monitor` - Detailed system monitoring with charts
- `/scripts` - Script execution with terminal output (Admin Only)
- `/history` - Task execution history
- `/settings` - Scripts directory + quick action management (Admin Only)
- `/plugins` - Plugin manager (planned: enable/disable, view logs)
- `/plugins/:id` - Plugin view (planned: one route per plugin)

## Quick Actions Flow
Dashboard -> click Quick Action -> **POST /api/quick-actions/:id/execute** -> navigates to `/history?highlight_task=<id>` (accessible to Clients)

## Type Imports
Project uses `verbatimModuleSyntax` in TypeScript. Always use:
```typescript
import { api } from '../lib/api';
import type { QuickAction } from '../lib/api';
```
Not:
```typescript
import { api, QuickAction } from '../lib/api'; // Error!
```

## Plugin System (Planned)

### Plugin Manager UI (`/plugins`)
- List all installed plugins
- Show name, version, status (running/stopped/crashed), health indicator
- Enable/disable toggle per plugin
- View plugin details (author, description)
- View plugin logs (formatted, readable)
- Toast notifications on actions

### Plugin View (`/plugins/:id`)
- Dynamic route per plugin (registered in metadata)
- Load plugin's JavaScript bundle from `/api/plugins/:id/bundle.js`
- Call `window.ToruPlugins[id].mount(container, api)` on mount
- Call `window.ToruPlugins[id].unmount(container)` on unmount
- Provide API object: `{ fetch, navigate, showToast }`
- Plugin has FULL CONTROL inside its container
- WordPress-style: one sidebar button, one view, full freedom

### Sidebar Integration
- Fetch enabled plugins on app load
- Add plugin entries below system items
- Show plugin icon and name from metadata
- Health indicator (green/red dot) for each plugin
- Hide plugins section when no plugins enabled
- Plugin routes register dynamically in React Router

### API Client Functions (to add in lib/api.ts)
```typescript
// Plugin management
listPlugins(): Promise<Plugin[]>
getPlugin(id: string): Promise<Plugin>
enablePlugin(id: string): Promise<void>
disablePlugin(id: string): Promise<void>
getPluginLogs(id: string, page: number): Promise<PluginLogs>

// Plugin frontend
fetchPluginBundle(id: string): Promise<string>
```

### Plugin Frontend Contract (for plugin authors)
```javascript
window.ToruPlugins = window.ToruPlugins || {};
window.ToruPlugins["my-plugin-id"] = {
    mount(container, api) {
        // container: DOM element to render into
        // api: { fetch, navigate, showToast }
        // Use React, Vue, vanilla JS, anything
        // Full freedom inside container
    },
    unmount(container) {
        // Cleanup event listeners, timers, etc.
    }
};
```

### Shadcn/ui Components to Use
- `Card` - Plugin cards in manager
- `Button` - Enable/disable toggles
- `Badge` - Status indicators
- `ScrollArea` - Log viewer
- `Dialog` - Plugin details/logs modal
- `Progress` - Loading states
- `Tooltip` - Status explanations
