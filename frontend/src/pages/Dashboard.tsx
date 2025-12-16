import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSystemStats } from '../hooks/useSystemStats';
import { formatUptime, formatBytes } from '../lib/utils';
import { api } from '../lib/api';
import type { QuickAction } from '../lib/api';
import { 
  Cpu, 
  MemoryStick, 
  Clock, 
  HardDrive,
  AlertCircle,
  RefreshCw,
  Zap,
  Play,
  ChevronRight,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

export function Dashboard() {
  const { stats, loading, error } = useSystemStats(5000);
  const [quickActions, setQuickActions] = useState<QuickAction[]>([]);
  const [loadingActions, setLoadingActions] = useState(true);
  const navigate = useNavigate();

  useEffect(() => {
    const fetchQuickActions = async () => {
      try {
        const actions = await api.getQuickActions();
        setQuickActions(actions);
      } catch (err) {
        console.error('Failed to fetch quick actions:', err);
      } finally {
        setLoadingActions(false);
      }
    };
    fetchQuickActions();
  }, []);

  const handleRunQuickAction = async (action: QuickAction) => {
    try {
      setLoadingActions(true);
      const result = await api.runQuickAction(action.id);
      // Navigate to history page with specific task highlighted
      navigate(`/history?highlight_task=${result.task_id}`);
    } catch (err) {
      console.error('Failed to run action:', err);
      // Maybe show toast error here?
      setLoadingActions(false);
    }
  };

  if (error) {
    console.error('Dashboard: Error loading system stats', error);
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="text-4xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-muted-foreground">
            System overview at a glance
          </p>
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

        <Card>
          <CardHeader>
            <CardTitle>Troubleshooting</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm">
            <p>Possible causes:</p>
            <ul className="list-disc list-inside space-y-1 text-muted-foreground ml-2">
              <li>The backend server is not running</li>
              <li>The API endpoint is not accessible</li>
              <li>Network connection issues</li>
              <li>CORS or firewall blocking the request</li>
            </ul>
            <p className="mt-4">Check the terminal running the server for error messages.</p>
          </CardContent>
        </Card>
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

  // Calculate total disk usage across all disks
  const totalDiskSpace = stats.disks.reduce((acc, disk) => acc + disk.total_space, 0);
  const totalDiskUsed = stats.disks.reduce((acc, disk) => acc + disk.used_space, 0);
  const totalDiskPercent = totalDiskSpace > 0 ? (totalDiskUsed / totalDiskSpace) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <h1 className="text-4xl font-bold tracking-tight">Dashboard</h1>
        <p className="text-muted-foreground">
          System overview at a glance
        </p>
      </div>

      {/* Key Metrics Grid - Single row, compact layout */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {/* CPU Usage */}
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

        {/* Memory Storage (Disk) */}
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Storage</CardTitle>
            <HardDrive className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{totalDiskPercent.toFixed(1)}%</div>
            <Progress value={totalDiskPercent} className="mt-2" />
            <p className="text-xs text-muted-foreground mt-2">
              {formatBytes(totalDiskUsed)} / {formatBytes(totalDiskSpace)}
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Zap className="h-5 w-5" />
              <CardTitle>Quick Actions</CardTitle>
            </div>
            <Button 
              variant="ghost" 
              size="sm" 
              onClick={() => navigate('/settings')}
              className="text-muted-foreground"
            >
              Manage
              <ChevronRight className="h-4 w-4 ml-1" />
            </Button>
          </div>
          <CardDescription>
            Execute frequently used scripts with one click
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadingActions ? (
            <div className="flex items-center justify-center py-8">
              <RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />
            </div>
          ) : quickActions.length === 0 ? (
            <div className="rounded-lg border border-dashed p-8 text-center">
              <Zap className="mx-auto h-8 w-8 text-muted-foreground opacity-50 mb-2" />
              <p className="text-sm text-muted-foreground mb-3">
                No quick actions configured yet.
              </p>
              <Button 
                variant="outline" 
                size="sm"
                onClick={() => navigate('/settings')}
              >
                Add Quick Action
              </Button>
            </div>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {quickActions.map((action) => (
                <Button
                  key={action.id}
                  variant="outline"
                  className="h-auto py-4 px-4 justify-start"
                  onClick={() => handleRunQuickAction(action)}
                >
                  <Play className="h-4 w-4 mr-3 text-primary" />
                  <div className="text-left">
                    <div className="font-medium">{action.name}</div>
                    <div className="text-xs text-muted-foreground truncate max-w-[150px]">
                      {action.script_path}
                    </div>
                  </div>
                </Button>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
