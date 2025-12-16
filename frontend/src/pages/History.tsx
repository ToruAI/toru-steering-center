import { useEffect, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { api } from '../lib/api';
import type { TaskHistory } from '../lib/api';
import { 
  History as HistoryIcon,
  RefreshCw, 
  CheckCircle2, 
  XCircle, 
  Clock,
  Play,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';

export function History() {
  const [history, setHistory] = useState<TaskHistory[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const highlightTask = searchParams.get('highlight_task');

  const fetchHistory = async () => {
    setLoading(true);
    try {
      const data = await api.getHistory();
      setHistory(data);
      // Auto-expand highlighted task if present in list
      if (highlightTask && data.some(t => t.id === highlightTask)) {
        setExpandedId(highlightTask);
      }
    } catch (err) {
      console.error('Failed to fetch history:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchHistory();
    // Set up polling for updates if looking at a specific task
    let interval: number;
    if (highlightTask) {
        interval = setInterval(async () => {
            const data = await api.getHistory();
            setHistory(data);
             // Verify it's still running or just to update output
        }, 2000) as unknown as number;
    }
    return () => clearInterval(interval);
  }, [highlightTask]); // Re-run when highlightTask changes


  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleString();
  };

  const getStatusBadge = (task: TaskHistory) => {
    if (task.finished_at === null) {
      return (
        <Badge variant="default" className="animate-pulse">
          <Clock className="h-3 w-3 mr-1" />
          Running
        </Badge>
      );
    }
    if (task.exit_code === 0) {
      return (
        <Badge variant="default" className="bg-green-600">
          <CheckCircle2 className="h-3 w-3 mr-1" />
          Success
        </Badge>
      );
    }
    return (
      <Badge variant="destructive">
        <XCircle className="h-3 w-3 mr-1" />
        Failed ({task.exit_code})
      </Badge>
    );
  };

  const handleRerun = (scriptName: string) => {
    navigate(`/scripts?script=${encodeURIComponent(scriptName)}`);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-muted-foreground flex items-center gap-2">
          <RefreshCw className="h-4 w-4 animate-spin" />
          Loading history...
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-4xl font-bold tracking-tight">History</h1>
          <p className="text-muted-foreground mt-2">
            View past script executions and their results
          </p>
        </div>
        <Button variant="outline" onClick={fetchHistory}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Refresh
        </Button>
      </div>

      {/* History List */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <HistoryIcon className="h-5 w-5" />
            <CardTitle>Execution History</CardTitle>
          </div>
          <CardDescription>
            Last 100 script executions
          </CardDescription>
        </CardHeader>
        <CardContent>
          {history.length === 0 ? (
            <div className="rounded-lg border border-dashed p-8 text-center">
              <HistoryIcon className="mx-auto h-8 w-8 text-muted-foreground opacity-50 mb-2" />
              <p className="text-sm text-muted-foreground">
                No execution history yet. Run a script to see it here.
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {history.map((task) => (
                <div
                  key={task.id}
                  className="rounded-lg border overflow-hidden"
                >
                  {/* Task Header */}
                  <div 
                    className="flex items-center justify-between p-4 hover:bg-accent/50 transition-colors cursor-pointer"
                    onClick={() => setExpandedId(expandedId === task.id ? null : task.id)}
                  >
                    <div className="flex items-center gap-3 flex-1 min-w-0">
                      <div className="flex-1 min-w-0">
                        <p className="font-medium truncate">{task.script_name}</p>
                        <p className="text-xs text-muted-foreground">
                          {formatDate(task.started_at)}
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 ml-2">
                      {getStatusBadge(task)}
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleRerun(task.script_name);
                        }}
                        title="Run again"
                      >
                        <Play className="h-4 w-4" />
                      </Button>
                      {expandedId === task.id ? (
                        <ChevronUp className="h-4 w-4 text-muted-foreground" />
                      ) : (
                        <ChevronDown className="h-4 w-4 text-muted-foreground" />
                      )}
                    </div>
                  </div>

                  {/* Expanded Output */}
                  {expandedId === task.id && (
                    <div className="border-t bg-slate-950 p-4">
                      <div className="flex items-center justify-between mb-2 text-xs text-muted-foreground">
                        <span>Task ID: {task.id}</span>
                        {task.finished_at && (
                          <span>Finished: {formatDate(task.finished_at)}</span>
                        )}
                      </div>
                      <ScrollArea className="h-[200px]">
                        <pre className="text-sm font-mono text-slate-50 whitespace-pre-wrap break-words">
                          {task.output || '(No output captured)'}
                        </pre>
                      </ScrollArea>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
