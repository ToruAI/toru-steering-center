import { useEffect, useState, useRef } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useWebSocket } from '../hooks/useWebSocket';
import { api } from '../lib/api';
import { Play, Square, Terminal as TerminalIcon, Loader2, WifiOff } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { ScrollArea } from '@/components/ui/scroll-area';

export function Scripts() {
  const [searchParams] = useSearchParams();
  const [scripts, setScripts] = useState<string[]>([]);
  const [selectedScript, setSelectedScript] = useState<string>('');
  const [currentTaskId, setCurrentTaskId] = useState<string | null>(null);
  const terminalRef = useRef<HTMLDivElement>(null);
  
  const wsUrl = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/api/ws`;
  const { connected, messages, send, clearMessages } = useWebSocket(wsUrl);

  useEffect(() => {
    const fetchScripts = async () => {
      try {
        const scriptList = await api.listScripts();
        setScripts(scriptList);
        
        const scriptParam = searchParams.get('script');
        if (scriptParam) {
          const scriptName = scriptParam.split('/').pop() || scriptParam;
          if (scriptList.includes(scriptName)) {
            setSelectedScript(scriptName);
          }
        }
      } catch (err) {
        console.error('Failed to fetch scripts:', err);
      }
    };

    fetchScripts();
  }, [searchParams]);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [messages]);

  useEffect(() => {
    messages.forEach((msg) => {
      if (msg.type === 'started' && msg.task_id) {
        setCurrentTaskId(msg.task_id);
      } else if (msg.type === 'exit' || msg.type === 'cancelled' || msg.type === 'error') {
        setCurrentTaskId(null);
      }
    });
  }, [messages]);

  const handleRun = () => {
    if (!selectedScript) return;
    clearMessages();
    send({ type: 'run', script: selectedScript });
  };

  const handleCancel = () => {
    if (currentTaskId) {
      send({ type: 'cancel', task_id: currentTaskId });
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-4xl font-bold tracking-tight">Scripts</h1>
        <p className="text-muted-foreground mt-2">
          Execute and monitor scripts with real-time output
        </p>
      </div>

      {/* Controls Card */}
      <Card>
        <CardHeader>
          <CardTitle>Script Execution</CardTitle>
          <CardDescription>
            Select a script and execute it with live terminal feedback
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-col sm:flex-row gap-3">
            <Select 
              value={selectedScript} 
              onValueChange={setSelectedScript}
              disabled={!!currentTaskId}
            >
              <SelectTrigger className="flex-1">
                <SelectValue placeholder="Select a script..." />
              </SelectTrigger>
              <SelectContent>
                {scripts.map((script) => (
                  <SelectItem key={script} value={script}>
                    {script}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            
            <div className="flex gap-2">
              <Button
                onClick={handleRun}
                disabled={!selectedScript || !!currentTaskId || !connected}
                size="default"
              >
                {currentTaskId ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Running
                  </>
                ) : (
                  <>
                    <Play className="mr-2 h-4 w-4" />
                    Run
                  </>
                )}
              </Button>
              
              {currentTaskId && (
                <Button
                  onClick={handleCancel}
                  variant="destructive"
                >
                  <Square className="mr-2 h-4 w-4" />
                  Cancel
                </Button>
              )}
            </div>
          </div>

          {!connected && (
            <Alert variant="destructive">
              <WifiOff className="h-4 w-4" />
              <AlertDescription>
                WebSocket disconnected. Attempting to reconnect...
              </AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>

      {/* Terminal Card */}
      <Card className="overflow-hidden">
        <CardHeader className="bg-muted/50 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <TerminalIcon className="h-4 w-4" />
              <CardTitle className="text-base">Terminal Output</CardTitle>
            </div>
            <div className="flex items-center gap-2">
              {currentTaskId && (
                <Badge variant="default" className="animate-pulse">
                  Running
                </Badge>
              )}
              {connected ? (
                <Badge variant="outline" className="text-xs">
                  Connected
                </Badge>
              ) : (
                <Badge variant="destructive" className="text-xs">
                  Disconnected
                </Badge>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent className="p-0">
          <ScrollArea className="h-[500px]">
            <div
              ref={terminalRef}
              className="p-4 font-mono text-sm bg-slate-950 text-slate-50 min-h-[500px]"
            >
              {messages.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-[468px] text-muted-foreground">
                  <TerminalIcon className="h-12 w-12 mb-4 opacity-20" />
                  <p>No output yet. Select a script and click Run.</p>
                </div>
              ) : (
                <div className="space-y-0.5">
                  {messages.map((msg, idx) => {
                    if (msg.type === 'stdout') {
                      return (
                        <div key={idx} className="text-green-400 whitespace-pre-wrap break-words">
                          {msg.data}
                        </div>
                      );
                    } else if (msg.type === 'stderr') {
                      return (
                        <div key={idx} className="text-red-400 whitespace-pre-wrap break-words">
                          {msg.data}
                        </div>
                      );
                    } else if (msg.type === 'started') {
                      return (
                        <div key={idx} className="text-blue-400 font-semibold">
                          ▶ Started - Task ID: {msg.task_id}
                        </div>
                      );
                    } else if (msg.type === 'exit') {
                      return (
                        <div key={idx} className={msg.code === 0 ? 'text-green-400 font-semibold' : 'text-red-400 font-semibold'}>
                          {msg.code === 0 ? '✓' : '✗'} Exit Code: {msg.code}
                        </div>
                      );
                    } else if (msg.type === 'cancelled') {
                      return (
                        <div key={idx} className="text-yellow-400 font-semibold">
                          ⚠ Cancelled
                        </div>
                      );
                    } else if (msg.type === 'error') {
                      return (
                        <div key={idx} className="text-red-400 font-semibold">
                          ✗ Error: {msg.data}
                        </div>
                      );
                    }
                    return null;
                  })}
                </div>
              )}
            </div>
          </ScrollArea>
        </CardContent>
      </Card>
    </div>
  );
}
