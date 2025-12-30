import { useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Loader2, AlertCircle } from 'lucide-react';

interface PluginMountFunction {
  mount: (container: HTMLElement, api: PluginAPI) => void;
  unmount: (container: HTMLElement) => void;
}

interface PluginAPI {
  fetch: typeof fetch;
  navigate: (path: string) => void;
  showToast: (message: string, type?: 'success' | 'error' | 'info') => void;
}

export function PluginView() {
  const { pluginId } = useParams<{ pluginId: string }>();
  const navigate = useNavigate();
  const containerRef = useRef<HTMLDivElement>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pluginMount, setPluginMount] = useState<PluginMountFunction | null>(null);

  // Plugin API provided to the plugin
  const pluginAPI: PluginAPI = {
    fetch: window.fetch.bind(window),
    navigate,
    showToast: (message, type = 'info') => {
      // Simple toast implementation using window events
      window.dispatchEvent(
        new CustomEvent('plugin-toast', {
          detail: { message, type },
        })
      );
    },
  };

  useEffect(() => {
    if (!pluginId) {
      setError('Plugin ID not provided');
      setLoading(false);
      return;
    }

    let scriptElement: HTMLScriptElement | null = null;

    const loadPlugin = async () => {
      try {
        setLoading(true);
        setError(null);

        // Load the plugin bundle
        const scriptUrl = `/api/plugins/${pluginId}/bundle.js`;

        // Create and load the script
        scriptElement = document.createElement('script');
        scriptElement.src = scriptUrl;
        scriptElement.async = true;

        // Wait for the script to load
        scriptElement.onload = () => {
          // The plugin should have exposed a global object with mount/unmount functions
          // Try both conventions: window.ToruPlugins[pluginId] and window.toru_plugin_{pluginId}
          const pluginGlobal = (window as any).ToruPlugins?.[pluginId] ||
                               (window as any)[`toru_plugin_${pluginId}`];

          if (!pluginGlobal || typeof pluginGlobal.mount !== 'function' || typeof pluginGlobal.unmount !== 'function') {
            throw new Error('Plugin bundle is invalid: missing mount/unmount functions');
          }

          setPluginMount({
            mount: pluginGlobal.mount,
            unmount: pluginGlobal.unmount,
          });

          setLoading(false);
        };

        scriptElement.onerror = () => {
          throw new Error('Failed to load plugin bundle');
        };

        document.head.appendChild(scriptElement);
      } catch (err) {
        console.error('Failed to load plugin:', err);
        setError(err instanceof Error ? err.message : 'Unknown error loading plugin');
        setLoading(false);
      }
    };

    loadPlugin();

    return () => {
      // Clean up: unmount plugin and remove script
      if (pluginMount && containerRef.current) {
        try {
          pluginMount.unmount(containerRef.current);
        } catch (err) {
          console.error('Failed to unmount plugin:', err);
        }
      }
      if (scriptElement && scriptElement.parentNode) {
        scriptElement.parentNode.removeChild(scriptElement);
      }
    };
  }, [pluginId]);

  // Mount the plugin when the container is ready and plugin is loaded
  useEffect(() => {
    if (pluginMount && containerRef.current && !loading && !error) {
      try {
        pluginMount.mount(containerRef.current, pluginAPI);
      } catch (err) {
        console.error('Failed to mount plugin:', err);
        setError(err instanceof Error ? err.message : 'Failed to mount plugin');
      }
    }
  }, [pluginMount, containerRef.current, loading, error]);

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center max-w-md">
          <AlertCircle className="h-12 w-12 mx-auto text-red-500 mb-4" />
          <h2 className="text-xl font-semibold mb-2">Plugin Error</h2>
          <p className="text-muted-foreground mb-4">{error}</p>
          <button
            onClick={() => navigate('/plugins')}
            className="text-primary hover:underline"
          >
            Back to Plugins
          </button>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <Loader2 className="h-8 w-8 animate-spin mx-auto text-muted-foreground mb-4" />
          <p className="text-muted-foreground">Loading plugin...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full">
      <div ref={containerRef} className="w-full h-full" />
    </div>
  );
}
