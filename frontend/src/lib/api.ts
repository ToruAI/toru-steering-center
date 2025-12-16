const API_BASE = '/api';

export interface CpuCore {
  name: string;
  usage: number;
}

export interface DiskInfo {
  name: string;
  mount_point: string;
  total_space: number;
  available_space: number;
  used_space: number;
  usage_percent: number;
}

export interface NetworkInterface {
  name: string;
  received: number;
  transmitted: number;
}

export interface SystemResources {
  cpu_percent: number;
  cpu_cores: CpuCore[];
  memory_percent: number;
  memory_used: number;
  memory_total: number;
  swap_used: number;
  swap_total: number;
  uptime_seconds: number;
  disks: DiskInfo[];
  network: NetworkInterface[];
  process_count: number;
  system_name: string | null;
  kernel_version: string | null;
  os_version: string | null;
  host_name: string | null;
}

export interface TaskHistory {
  id: string;
  script_name: string;
  started_at: string;
  finished_at: string | null;
  exit_code: number | null;
  output: string | null;
}

export interface QuickAction {
  id: string;
  name: string;
  script_path: string;
  icon: string | null;
  display_order: number;
}

export interface User {
  id: string;
  username: string;
  display_name: string | null;
  role: 'admin' | 'client';
  is_active: boolean;
  created_at: string;
}

export interface LoginResponse {
  success: boolean;
  user: {
    id: string | null;
    username: string;
    display_name: string | null;
    role: 'admin' | 'client';
  } | null;
  error: string | null;
  locked_until?: number;  // Seconds until lockout ends
}

export interface LoginAttempt {
  id: string;
  username: string;
  ip_address: string | null;
  success: boolean;
  failure_reason: string | null;
  attempted_at: string;
}

export interface CreateUserPayload {
  username: string;
  password: string;
  display_name?: string;
}

export interface UpdateUserPayload {
  display_name?: string;
  is_active?: boolean;
}

export interface MeResponse {
  authenticated: boolean;
  user: {
    id: string | null;
    username: string;
    display_name: string | null;
    role: 'admin' | 'client';
  } | null;
}

export interface Setting {
  key: string;
  value: string;
}

async function handleResponse<T>(res: Response, endpoint: string): Promise<T> {
  if (!res.ok) {
    const errorText = await res.text().catch(() => 'Unknown error');
    console.error(`API Error [${endpoint}]:`, {
      status: res.status,
      statusText: res.statusText,
      body: errorText,
      url: res.url,
    });
    throw new Error(`API request failed: ${res.status} ${res.statusText} - ${errorText}`);
  }
  
  try {
    return await res.json();
  } catch (err) {
    console.error(`JSON Parse Error [${endpoint}]:`, err);
    throw new Error(`Failed to parse JSON response from ${endpoint}`);
  }
}

// Global auth error handler - set by AuthContext
let onAuthError: (() => void) | null = null;

export function setAuthErrorHandler(handler: () => void) {
  onAuthError = handler;
}

async function handleAuthResponse<T>(res: Response, endpoint: string): Promise<T> {
  if (res.status === 401) {
    onAuthError?.();
    throw new Error('Session expired');
  }
  return handleResponse(res, endpoint);
}

// Helper for requests with credentials
async function request(endpoint: string, options: RequestInit = {}) {
  return fetch(`${API_BASE}${endpoint}`, {
    ...options,
    // IMPORTANT: Include cookies in all requests
    credentials: 'include', 
    headers: {
      ...options.headers,
    }
  });
}

// Helper for JSON requests
async function jsonRequest(endpoint: string, method: string, body?: any) {
  return request(endpoint, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
}

export const api = {
  // Auth endpoints
  login: async (username: string, password: string): Promise<LoginResponse> => {
    try {
      const res = await jsonRequest('/auth/login', 'POST', { username, password });
      return res.json();
    } catch (err) {
      return {
        success: false,
        user: null,
        error: 'Network error. Please check your connection.',
      };
    }
  },

  logout: async (): Promise<void> => {
    await request('/auth/logout', { method: 'POST' });
  },

  me: async (): Promise<MeResponse> => {
    const res = await request('/auth/me');
    return res.json();
  },

  getLoginHistory: async (): Promise<LoginAttempt[]> => {
    const res = await request('/auth/login-history');
    return handleAuthResponse(res, '/auth/login-history');
  },

  // User management (Admin only)
  listUsers: async (): Promise<User[]> => {
    const res = await request('/users');
    return handleAuthResponse(res, '/users');
  },

  createUser: async (data: CreateUserPayload): Promise<User> => {
    const res = await jsonRequest('/users', 'POST', data);
    return handleAuthResponse(res, '/users');
  },

  updateUser: async (id: string, data: UpdateUserPayload): Promise<User> => {
    const res = await jsonRequest(`/users/${id}`, 'PUT', data);
    return handleAuthResponse(res, `/users/${id}`);
  },

  deleteUser: async (id: string): Promise<void> => {
    const res = await request(`/users/${id}`, { method: 'DELETE' });
    if (res.status === 401) {
      onAuthError?.();
      throw new Error('Session expired');
    }
    if (!res.ok) throw new Error('Failed to delete user');
  },

  resetPassword: async (id: string, password: string): Promise<void> => {
    const res = await jsonRequest(`/users/${id}/password`, 'PUT', { password });
    if (res.status === 401) {
      onAuthError?.();
      throw new Error('Session expired');
    }
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to reset password');
    }
  },

  changeOwnPassword: async (currentPassword: string, newPassword: string): Promise<void> => {
    const res = await jsonRequest('/me/password', 'PUT', { 
      current_password: currentPassword, 
      new_password: newPassword 
    });
    if (res.status === 401) {
      onAuthError?.();
      throw new Error('Session expired');
    }
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.error || 'Failed to change password');
    }
  },

  health: async (): Promise<{ status: string }> => {
    const res = await fetch(`${API_BASE}/health`); // Health can be public/no-auth-cookies if needed, but safe to include
    return handleResponse(res, '/health');
  },

  getResources: async (): Promise<SystemResources> => {
    const res = await request('/resources');
    return handleAuthResponse(res, '/resources');
  },

  listScripts: async (): Promise<string[]> => {
    const res = await request('/scripts');
    return handleAuthResponse(res, '/scripts');
  },

  getSettings: async (): Promise<{ settings: Setting[] }> => {
    const res = await request('/settings');
    return handleAuthResponse(res, '/settings');
  },

  updateSetting: async (key: string, value: string): Promise<void> => {
    const res = await jsonRequest(`/settings/${key}`, 'PUT', { value });
    await handleAuthResponse(res, `/settings/${key}`);
  },

  getHistory: async (): Promise<TaskHistory[]> => {
    const res = await request('/history');
    return handleAuthResponse(res, '/history');
  },

  getQuickActions: async (): Promise<QuickAction[]> => {
    const res = await request('/quick-actions');
    return handleAuthResponse(res, '/quick-actions');
  },

  createQuickAction: async (action: Omit<QuickAction, 'id'>): Promise<QuickAction> => {
    const res = await jsonRequest('/quick-actions', 'POST', action);
    return handleAuthResponse(res, '/quick-actions');
  },

  runQuickAction: async (id: string): Promise<{ task_id: string }> => {
    const res = await request(`/quick-actions/${id}/execute`, { method: 'POST' });
    return handleAuthResponse(res, `/quick-actions/${id}/execute`);
  },

  deleteQuickAction: async (id: string): Promise<void> => {
    const res = await request(`/quick-actions/${id}`, { method: 'DELETE' });
    await handleAuthResponse(res, `/quick-actions/${id}`);
  },
};
