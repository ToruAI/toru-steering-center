import { useState, useEffect } from 'react';
import { useSystemStats } from '../hooks/useSystemStats';
import { formatUptime, formatBytes } from '../lib/utils';
import { 
  Cpu, 
  MemoryStick, 
  Clock, 
  HardDrive, 
  Network, 
  Activity,
  Layers,
  AlertCircle,
  RefreshCw,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Area,
  AreaChart,
} from 'recharts';

interface CpuHistoryPoint {
  time: string;
  usage: number;
  timestamp?: number;
}

export function SystemMonitor() {
  const { stats, loading, error } = useSystemStats(2000);
  const [cpuHistory, setCpuHistory] = useState<CpuHistoryPoint[]>(() => {
    // Load cached CPU history from localStorage on mount
    try {
      const cached = localStorage.getItem('cpuHistory');
      if (cached) {
        const parsed = JSON.parse(cached);
        // Validate and filter out stale data (older than 1 hour)
        const oneHourAgo = Date.now() - 60 * 60 * 1000;
        return parsed.filter((point: CpuHistoryPoint & { timestamp: number }) => 
          point.timestamp && point.timestamp > oneHourAgo
        );
      }
    } catch (error) {
      console.error('Failed to load CPU history from cache:', error);
    }
    return [];
  });

  // Track CPU history for the chart
  useEffect(() => {
    if (stats) {
      const now = new Date();
      const timestamp = now.getTime();
      const timeStr = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`;
      
      setCpuHistory((prev) => {
        const newHistory = [
          ...prev,
          { time: timeStr, usage: stats.cpu_percent, timestamp } as CpuHistoryPoint & { timestamp: number }
        ];
        
        // Keep only last 1800 data points (1 hour of history at 2s intervals)
        const trimmedHistory = newHistory.slice(-1800);
        
        // Cache to localStorage
        try {
          localStorage.setItem('cpuHistory', JSON.stringify(trimmedHistory));
        } catch (error) {
          console.error('Failed to cache CPU history:', error);
        }
        
        return trimmedHistory;
      });
    }
  }, [stats]);

  const getUsageVariant = (percent: number) => {
    if (percent >= 90) return 'destructive';
    if (percent >= 70) return 'secondary';
    return 'default';
  };

  if (error) {
    console.error('SystemMonitor: Error loading system stats', error);
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="text-4xl font-bold tracking-tight">System Monitor</h1>
        </div>
        
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Failed to load system information</AlertTitle>
          <AlertDescription className="mt-2 space-y-2">
            <p className="text-sm">{error.message}</p>
            <p className="text-xs text-muted-foreground mt-2">
              Please check the browser console (F12) for more details.
            </p>
            <Button 
              variant="outline" 
              size="sm" 
              onClick={() => window.location.reload()}
              className="mt-2"
            >
              <RefreshCw className="h-4 w-4 mr-2" />
              Retry
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  if (loading || !stats) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-muted-foreground flex items-center gap-2">
          <RefreshCw className="h-4 w-4 animate-spin" />
          Loading system information...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with System Info */}
      <div className="space-y-2">
        <h1 className="text-4xl font-bold tracking-tight">System Monitor</h1>
        <div className="flex flex-wrap gap-2 text-sm text-muted-foreground">
          {stats.host_name && <span>{stats.host_name}</span>}
          {stats.system_name && (
            <>
              <span>•</span>
              <span>{stats.system_name}</span>
            </>
          )}
          {stats.os_version && (
            <>
              <span>•</span>
              <span>{stats.os_version}</span>
            </>
          )}
          {stats.kernel_version && (
            <>
              <span>•</span>
              <span>Kernel {stats.kernel_version}</span>
            </>
          )}
        </div>
      </div>

      {/* Key Metrics Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {/* CPU Overview */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">CPU Usage</CardTitle>
            <Cpu className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.cpu_percent.toFixed(1)}%</div>
            <Progress value={stats.cpu_percent} className="mt-2" />
            <p className="text-xs text-muted-foreground mt-2">
              {stats.cpu_cores.length} cores
            </p>
          </CardContent>
        </Card>

        {/* Memory */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Memory</CardTitle>
            <MemoryStick className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.memory_percent.toFixed(1)}%</div>
            <Progress value={stats.memory_percent} className="mt-2" />
            <p className="text-xs text-muted-foreground mt-2">
              {formatBytes(stats.memory_used)} / {formatBytes(stats.memory_total)}
            </p>
          </CardContent>
        </Card>

        {/* Uptime */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Uptime</CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{formatUptime(stats.uptime_seconds)}</div>
            <p className="text-xs text-muted-foreground mt-2">
              System running
            </p>
          </CardContent>
        </Card>

        {/* Processes */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Processes</CardTitle>
            <Activity className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.process_count}</div>
            <p className="text-xs text-muted-foreground mt-2">
              Active processes
            </p>
          </CardContent>
        </Card>
      </div>

      {/* CPU Usage Chart */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Cpu className="h-5 w-5" />
            CPU Usage Over Time
          </CardTitle>
          <CardDescription>Real-time CPU usage (last hour)</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="h-[250px]">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={cpuHistory}>
                <defs>
                  <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3}/>
                    <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                <XAxis 
                  dataKey="time" 
                  className="text-xs"
                  tick={{ fill: 'hsl(var(--muted-foreground))' }}
                />
                <YAxis 
                  domain={[0, 100]}
                  className="text-xs"
                  tick={{ fill: 'hsl(var(--muted-foreground))' }}
                  label={{ value: '%', position: 'insideLeft', style: { fill: 'hsl(var(--muted-foreground))' } }}
                />
                <Tooltip 
                  contentStyle={{
                    backgroundColor: 'hsl(var(--card))',
                    border: '1px solid hsl(var(--border))',
                    borderRadius: '6px',
                  }}
                />
                <Area 
                  type="monotone" 
                  dataKey="usage" 
                  stroke="hsl(var(--primary))" 
                  fill="url(#cpuGradient)"
                  strokeWidth={2}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-2">
        {/* Per-Core CPU Usage */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Layers className="h-5 w-5" />
              CPU Cores
            </CardTitle>
            <CardDescription>Individual core usage</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="h-[300px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={stats.cpu_cores}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis 
                    dataKey="name" 
                    className="text-xs"
                    tick={{ fill: 'hsl(var(--muted-foreground))' }}
                  />
                  <YAxis 
                    domain={[0, 100]}
                    className="text-xs"
                    tick={{ fill: 'hsl(var(--muted-foreground))' }}
                    label={{ value: '%', position: 'insideLeft', style: { fill: 'hsl(var(--muted-foreground))' } }}
                  />
                  <Tooltip 
                    contentStyle={{
                      backgroundColor: 'hsl(var(--card))',
                      border: '1px solid hsl(var(--border))',
                      borderRadius: '6px',
                    }}
                  />
                  <Bar dataKey="usage" fill="hsl(var(--primary))" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Memory Details */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <MemoryStick className="h-5 w-5" />
              Memory Details
            </CardTitle>
            <CardDescription>RAM and swap usage</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* RAM */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">RAM</span>
                <Badge variant={getUsageVariant(stats.memory_percent)}>
                  {stats.memory_percent.toFixed(1)}%
                </Badge>
              </div>
              <Progress value={stats.memory_percent} className="h-3" />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{formatBytes(stats.memory_used)} used</span>
                <span>{formatBytes(stats.memory_total)} total</span>
              </div>
            </div>

            <Separator />

            {/* Swap */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Swap</span>
                <Badge variant={stats.swap_total > 0 ? getUsageVariant((stats.swap_used / stats.swap_total) * 100) : 'secondary'}>
                  {stats.swap_total > 0 ? ((stats.swap_used / stats.swap_total) * 100).toFixed(1) : '0.0'}%
                </Badge>
              </div>
              <Progress 
                value={stats.swap_total > 0 ? (stats.swap_used / stats.swap_total) * 100 : 0} 
                className="h-3" 
              />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{formatBytes(stats.swap_used)} used</span>
                <span>{formatBytes(stats.swap_total)} total</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Disk Usage */}
      {stats.disks.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HardDrive className="h-5 w-5" />
              Storage
            </CardTitle>
            <CardDescription>Disk usage by mount point</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {stats.disks.map((disk, idx) => (
              <div key={idx} className="space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">{disk.mount_point}</p>
                    <p className="text-xs text-muted-foreground truncate">{disk.name}</p>
                  </div>
                  <Badge variant={getUsageVariant(disk.usage_percent)} className="ml-2">
                    {disk.usage_percent.toFixed(1)}%
                  </Badge>
                </div>
                <Progress value={disk.usage_percent} className="h-2" />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>{formatBytes(disk.used_space)} used</span>
                  <span>{formatBytes(disk.total_space)} total</span>
                </div>
                {idx < stats.disks.length - 1 && <Separator className="mt-4" />}
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Network Interfaces */}
      {stats.network.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Network className="h-5 w-5" />
              Network Interfaces
            </CardTitle>
            <CardDescription>Total data transferred since boot</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {stats.network.map((iface, idx) => (
                <div key={idx} className="flex items-center justify-between p-3 rounded-lg border">
                  <div className="flex-1">
                    <p className="text-sm font-medium">{iface.name}</p>
                    <div className="flex gap-4 mt-1 text-xs text-muted-foreground">
                      <span>↓ {formatBytes(iface.received)}</span>
                      <span>↑ {formatBytes(iface.transmitted)}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}


