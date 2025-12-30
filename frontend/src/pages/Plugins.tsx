import { useEffect, useState } from 'react';
import { api } from '../lib/api';
import type { Plugin, PluginLogEntry } from '../lib/api';
import { Plug2, Loader2, X, FileText } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { useAuth } from '@/contexts/AuthContext';

export function Plugins() {
  const { isAdmin } = useAuth();
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [logs, setLogs] = useState<PluginLogEntry[]>([]);
  const [loadingLogs, setLoadingLogs] = useState(false);
  const [togglingId, setTogglingId] = useState<string | null>(null);

  useEffect(() => {
    const fetchPlugins = async () => {
      if (!isAdmin) {
        setLoading(false);
        return;
      }

      try {
        const data = await api.listPlugins();
        setPlugins(data);
      } catch (err) {
        console.error('Failed to fetch plugins:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchPlugins();
  }, [isAdmin]);

  const handleTogglePlugin = async (plugin: Plugin) => {
    setTogglingId(plugin.id);
    try {
      if (plugin.enabled) {
        await api.disablePlugin(plugin.id);
      } else {
        await api.enablePlugin(plugin.id);
      }
      // Refresh the list
      const data = await api.listPlugins();
      setPlugins(data);
    } catch (err) {
      console.error('Failed to toggle plugin:', err);
      alert(`Failed to ${plugin.enabled ? 'disable' : 'enable'} plugin`);
    } finally {
      setTogglingId(null);
    }
  };

  const handleShowLogs = async (plugin: Plugin) => {
    setSelectedPlugin(plugin);
    setLoadingLogs(true);
    try {
      const data = await api.getPluginLogs(plugin.id);
      setLogs(data.logs);
    } catch (err) {
      console.error('Failed to fetch plugin logs:', err);
    } finally {
      setLoadingLogs(false);
    }
  };

  const getHealthBadgeVariant = (health: string): 'default' | 'secondary' | 'destructive' | 'outline' => {
    switch (health) {
      case 'healthy':
        return 'default';
      case 'unhealthy':
        return 'destructive';
      case 'disabled':
        return 'secondary';
      default:
        return 'outline';
    }
  };

  if (!isAdmin) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Plug2 className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
          <p className="text-muted-foreground">Plugin management is admin only</p>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <h1 className="text-4xl font-bold tracking-tight">Plugins</h1>
        <p className="text-muted-foreground">
          Manage and monitor installed plugins
        </p>
      </div>

      {/* Plugins List */}
      {plugins.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Plug2 className="h-16 w-16 text-muted-foreground opacity-20 mb-4" />
            <p className="text-lg font-medium mb-2">No plugins found</p>
            <p className="text-sm text-muted-foreground text-center max-w-md">
              Place plugin binaries (.binary files) in the ./plugins/ directory to make them available.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {plugins.map((plugin) => (
            <Card key={plugin.id} className="hover:shadow-md transition-shadow">
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3 flex-1">
                    {plugin.icon ? (
                      <div className="h-10 w-10 rounded-lg bg-accent flex items-center justify-center text-xl">
                        {plugin.icon}
                      </div>
                    ) : (
                      <div className="h-10 w-10 rounded-lg bg-primary/10 flex items-center justify-center">
                        <Plug2 className="h-5 w-5 text-primary" />
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <CardTitle className="text-lg truncate">{plugin.name}</CardTitle>
                      <CardDescription className="truncate">
                        v{plugin.version} by {plugin.author}
                      </CardDescription>
                    </div>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Status */}
                <div className="flex items-center justify-between">
                  <Badge variant={getHealthBadgeVariant(plugin.health)}>
                    {plugin.health}
                  </Badge>
                  {plugin.running && (
                    <span className="text-xs text-muted-foreground">
                      PID: {plugin.pid || 'N/A'}
                    </span>
                  )}
                </div>

                {/* Toggle */}
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {plugin.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                  <Switch
                    checked={plugin.enabled}
                    onCheckedChange={() => handleTogglePlugin(plugin)}
                    disabled={togglingId === plugin.id}
                  />
                </div>

                {/* Logs Button */}
                <Button
                  variant="outline"
                  className="w-full"
                  onClick={() => handleShowLogs(plugin)}
                  disabled={!plugin.enabled || !plugin.running}
                >
                  <FileText className="h-4 w-4 mr-2" />
                  View Logs
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Logs Dialog */}
      <Dialog open={!!selectedPlugin} onOpenChange={(open) => !open && setSelectedPlugin(null)}>
        <DialogContent className="max-w-3xl max-h-[80vh]">
          <DialogHeader>
            <div className="flex items-center justify-between">
              <div>
                <DialogTitle>
                  {selectedPlugin?.name} - Logs
                </DialogTitle>
                <DialogDescription>
                  {selectedPlugin?.id}
                </DialogDescription>
              </div>
              <Button variant="ghost" size="icon" onClick={() => setSelectedPlugin(null)}>
                <X className="h-4 w-4" />
              </Button>
            </div>
          </DialogHeader>
          <Separator />
          <div className="h-[400px]">
            <ScrollArea className="h-full">
              {loadingLogs ? (
                <div className="flex items-center justify-center h-full">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : logs.length === 0 ? (
                <div className="flex items-center justify-center h-full text-muted-foreground">
                  No logs available
                </div>
              ) : (
                <div className="space-y-2 pr-4">
                  {logs.map((log, idx) => (
                    <div
                      key={idx}
                      className="rounded bg-muted/50 p-3 font-mono text-xs"
                    >
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-muted-foreground">{log.timestamp}</span>
                        <Badge variant="outline" className="text-xs">
                          {log.level}
                        </Badge>
                      </div>
                      <div className="text-foreground">{log.message}</div>
                      {log.error && (
                        <div className="text-red-500 mt-1">{log.error}</div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </ScrollArea>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
