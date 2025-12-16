import { useEffect, useState } from 'react';
import { api } from '../lib/api';
import type { LoginAttempt } from '../lib/api';
import { Shield, CheckCircle2, XCircle, Loader2, RefreshCw } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

export function LoginHistory() {
  const [attempts, setAttempts] = useState<LoginAttempt[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchAttempts = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.getLoginHistory();
      setAttempts(data);
    } catch (err) {
      setError('Failed to fetch login history');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchAttempts();
  }, []);

  const formatDate = (isoString: string) => {
    const date = new Date(isoString);
    return date.toLocaleString();
  };

  const recentFailures = attempts.filter(a => !a.success).slice(0, 5);
  const failureCount24h = attempts.filter(a => {
    const time = new Date(a.attempted_at).getTime();
    const now = Date.now();
    return !a.success && (now - time) < 24 * 60 * 60 * 1000;
  }).length;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            <CardTitle>Security & Login History</CardTitle>
          </div>
          <Button variant="outline" size="sm" onClick={fetchAttempts} disabled={loading}>
            <RefreshCw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
        <CardDescription>
          Monitor login attempts and security events
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Stats */}
        <div className="grid grid-cols-2 gap-4">
          <div className="rounded-lg border p-4">
            <p className="text-sm text-muted-foreground">Failed attempts (24h)</p>
            <p className={`text-2xl font-bold ${failureCount24h > 5 ? 'text-destructive' : ''}`}>
              {failureCount24h}
            </p>
          </div>
          <div className="rounded-lg border p-4">
            <p className="text-sm text-muted-foreground">Total logged</p>
            <p className="text-2xl font-bold">{attempts.length}</p>
          </div>
        </div>

        {/* Recent Failures Alert */}
        {recentFailures.length > 0 && failureCount24h > 3 && (
          <div className="rounded-lg border border-yellow-200 bg-yellow-50 p-4 dark:bg-yellow-950 dark:border-yellow-900">
            <p className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
              Multiple failed login attempts detected
            </p>
            <p className="text-xs text-yellow-700 dark:text-yellow-300 mt-1">
              Consider reviewing the attempts below for suspicious activity.
            </p>
          </div>
        )}

        {/* Login Attempts Table */}
        {loading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : error ? (
          <div className="text-center py-8 text-destructive">{error}</div>
        ) : attempts.length === 0 ? (
          <div className="text-center py-8 text-muted-foreground">
            No login attempts recorded yet.
          </div>
        ) : (
          <div className="rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Status</TableHead>
                  <TableHead>Username</TableHead>
                  <TableHead>IP Address</TableHead>
                  <TableHead>Time</TableHead>
                  <TableHead>Details</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {attempts.slice(0, 50).map((attempt) => (
                  <TableRow key={attempt.id}>
                    <TableCell>
                      {attempt.success ? (
                        <Badge variant="outline" className="text-green-600 border-green-300">
                          <CheckCircle2 className="h-3 w-3 mr-1" />
                          Success
                        </Badge>
                      ) : (
                        <Badge variant="outline" className="text-red-600 border-red-300">
                          <XCircle className="h-3 w-3 mr-1" />
                          Failed
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="font-mono text-sm">{attempt.username}</TableCell>
                    <TableCell className="font-mono text-sm text-muted-foreground">
                      {attempt.ip_address || 'N/A'}
                    </TableCell>
                    <TableCell className="text-sm">{formatDate(attempt.attempted_at)}</TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {attempt.failure_reason || '-'}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
