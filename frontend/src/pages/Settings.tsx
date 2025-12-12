import { useEffect, useState } from 'react';
import type { QuickAction, Setting } from '../lib/api';
import { api } from '../lib/api';
import { Plus, Trash2, FolderOpen, Zap, Save, Loader2 } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Alert, AlertDescription } from '@/components/ui/alert';

export function Settings() {
  const [_settings, setSettings] = useState<Setting[]>([]);
  const [quickActions, setQuickActions] = useState<QuickAction[]>([]);
  const [scriptsDir, setScriptsDir] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [newActionName, setNewActionName] = useState('');
  const [newActionScript, setNewActionScript] = useState('');
  const [scripts, setScripts] = useState<string[]>([]);
  const [saveSuccess, setSaveSuccess] = useState(false);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [settingsData, actions, scriptsList] = await Promise.all([
          api.getSettings(),
          api.getQuickActions(),
          api.listScripts(),
        ]);
        
        setSettings(settingsData.settings);
        setQuickActions(actions);
        setScripts(scriptsList);
        
        const scriptsDirSetting = settingsData.settings.find((s) => s.key === 'scripts_dir');
        if (scriptsDirSetting) {
          setScriptsDir(scriptsDirSetting.value);
        }
      } catch (err) {
        console.error('Failed to fetch settings:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  const handleSaveScriptsDir = async () => {
    setSaving(true);
    setSaveSuccess(false);
    try {
      await api.updateSetting('scripts_dir', scriptsDir);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err) {
      alert('Failed to save settings');
      console.error(err);
    } finally {
      setSaving(false);
    }
  };

  const handleAddQuickAction = async () => {
    if (!newActionName || !newActionScript) {
      return;
    }

    setSaving(true);
    try {
      const newAction = await api.createQuickAction({
        name: newActionName,
        script_path: newActionScript,
        icon: null,
        display_order: quickActions.length,
      });
      setQuickActions([...quickActions, newAction]);
      setNewActionName('');
      setNewActionScript('');
    } catch (err) {
      alert('Failed to create quick action');
      console.error(err);
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteQuickAction = async (id: string) => {
    try {
      await api.deleteQuickAction(id);
      setQuickActions(quickActions.filter((a) => a.id !== id));
    } catch (err) {
      alert('Failed to delete quick action');
      console.error(err);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-4xl font-bold tracking-tight">Settings</h1>
        <p className="text-muted-foreground mt-2">
          Configure application preferences and quick actions
        </p>
      </div>

      {/* Scripts Directory */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <FolderOpen className="h-5 w-5" />
            <CardTitle>Scripts Directory</CardTitle>
          </div>
          <CardDescription>
            Specify the directory where your executable scripts are stored
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="scripts-dir">Directory Path</Label>
            <div className="flex gap-3">
              <Input
                id="scripts-dir"
                type="text"
                value={scriptsDir}
                onChange={(e) => setScriptsDir(e.target.value)}
                placeholder="./scripts"
                className="flex-1"
              />
              <Button
                onClick={handleSaveScriptsDir}
                disabled={saving}
              >
                {saving ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Saving
                  </>
                ) : (
                  <>
                    <Save className="mr-2 h-4 w-4" />
                    Save
                  </>
                )}
              </Button>
            </div>
          </div>
          
          {saveSuccess && (
            <Alert className="bg-green-50 border-green-200 dark:bg-green-950 dark:border-green-900">
              <AlertDescription className="text-green-700 dark:text-green-400">
                Settings saved successfully!
              </AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-2">
            <Zap className="h-5 w-5" />
            <CardTitle>Quick Actions</CardTitle>
          </div>
          <CardDescription>
            Create shortcuts for frequently executed scripts
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Add New Quick Action */}
          <div className="rounded-lg border bg-card p-4 space-y-4">
            <h3 className="font-semibold text-sm">Add New Quick Action</h3>
            <div className="space-y-3">
              <div className="space-y-2">
                <Label htmlFor="action-name">Action Name</Label>
                <Input
                  id="action-name"
                  type="text"
                  value={newActionName}
                  onChange={(e) => setNewActionName(e.target.value)}
                  placeholder="e.g., Check Disk Usage"
                />
              </div>
              
              <div className="space-y-2">
                <Label htmlFor="action-script">Select Script</Label>
                <Select 
                  value={newActionScript} 
                  onValueChange={setNewActionScript}
                >
                  <SelectTrigger id="action-script">
                    <SelectValue placeholder="Choose a script..." />
                  </SelectTrigger>
                  <SelectContent>
                    {scripts.map((script) => (
                      <SelectItem key={script} value={script}>
                        {script}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <Button
                onClick={handleAddQuickAction}
                disabled={saving || !newActionName || !newActionScript}
                className="w-full sm:w-auto"
              >
                {saving ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Adding
                  </>
                ) : (
                  <>
                    <Plus className="mr-2 h-4 w-4" />
                    Add Quick Action
                  </>
                )}
              </Button>
            </div>
          </div>

          <Separator />

          {/* Quick Actions List */}
          <div className="space-y-3">
            <h3 className="font-semibold text-sm">Configured Actions</h3>
            {quickActions.length === 0 ? (
              <div className="rounded-lg border border-dashed p-8 text-center">
                <Zap className="mx-auto h-8 w-8 text-muted-foreground opacity-50 mb-2" />
                <p className="text-sm text-muted-foreground">
                  No quick actions configured yet.
                </p>
              </div>
            ) : (
              <div className="space-y-2">
                {quickActions.map((action) => (
                  <div
                    key={action.id}
                    className="flex items-center justify-between rounded-lg border p-4 hover:bg-accent/50 transition-colors"
                  >
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">{action.name}</p>
                      <p className="text-sm text-muted-foreground truncate">
                        {action.script_path}
                      </p>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDeleteQuickAction(action.id)}
                      className="ml-2 text-destructive hover:text-destructive hover:bg-destructive/10"
                    >
                      <Trash2 className="h-4 w-4" />
                      <span className="sr-only">Delete</span>
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
