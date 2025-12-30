import { Link, useLocation, Outlet } from 'react-router-dom';
import { Home, Terminal, Settings, Activity, History, LogOut, User, Menu, Plug2 } from 'lucide-react';
import { cn } from '../lib/utils';
import { Separator } from '@/components/ui/separator';
import { ToruLogo } from './ToruLogo';
import { useAuth } from '../contexts/AuthContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { useState, useEffect } from 'react';
import { api } from '../lib/api';
import type { Plugin } from '../lib/api';

export function Layout() {
  const location = useLocation();
  const { user, logout, isAdmin } = useAuth();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [enabledPlugins, setEnabledPlugins] = useState<Plugin[]>([]);

  // Fetch enabled plugins on mount (all authenticated users can see plugins)
  useEffect(() => {
    const fetchPlugins = async () => {
      if (!user) return; // Only fetch if authenticated
      try {
        const plugins = await api.listPlugins();
        // Filter for enabled and running plugins
        const activePlugins = plugins.filter(p => p.enabled && p.running);
        setEnabledPlugins(activePlugins);
      } catch (err) {
        console.error('Failed to fetch plugins:', err);
      }
    };

    fetchPlugins();
  }, [user]);

  const allNavItems = [
    { path: '/', icon: Home, label: 'Dashboard', public: true },
    { path: '/system-monitor', icon: Activity, label: 'System Monitor', public: true },
    { path: '/scripts', icon: Terminal, label: 'Scripts', adminOnly: true },
    { path: '/history', icon: History, label: 'History', public: true },
    { path: '/plugins', icon: Plug2, label: 'Plugins', adminOnly: true },
    { path: '/settings', icon: Settings, label: 'Settings', public: true },
  ];

  const navItems = allNavItems.filter(item => item.public || (item.adminOnly && isAdmin));

  const handleMobileNavClick = () => {
    setMobileMenuOpen(false);
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Mobile Header */}
      <header className="fixed top-0 left-0 right-0 bg-card border-b border-border md:hidden z-50 safe-area-inset-top">
        <div className="flex items-center justify-between h-14 px-4">
          <ToruLogo size="sm" showText={true} />
          <Sheet open={mobileMenuOpen} onOpenChange={setMobileMenuOpen}>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon">
                <Menu className="h-5 w-5" />
                <span className="sr-only">Open menu</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="right" className="w-[280px]">
              <SheetHeader>
                <SheetTitle>Menu</SheetTitle>
              </SheetHeader>
              <div className="flex flex-col h-full py-4">
                {/* User Info */}
                <div className="flex items-center gap-3 p-3 rounded-lg bg-accent/50 mb-4">
                  <div className="h-10 w-10 rounded-full bg-primary/20 flex items-center justify-center text-primary">
                    <User className="h-5 w-5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium truncate">{user?.display_name || user?.username}</p>
                    <p className="text-xs text-muted-foreground capitalize">{user?.role}</p>
                  </div>
                </div>
                
                <Separator className="mb-4" />
                
                {/* Navigation */}
                <nav className="flex-1 space-y-1">
                  {navItems.map((item) => {
                    const Icon = item.icon;
                    const isActive = location.pathname === item.path;
                    return (
                      <Link
                        key={item.path}
                        to={item.path}
                        onClick={handleMobileNavClick}
                        className={cn(
                          'flex items-center gap-3 rounded-lg px-3 py-3 text-sm font-medium transition-all',
                          isActive
                            ? 'bg-primary text-primary-foreground'
                            : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                        )}
                      >
                        <Icon className="h-5 w-5" />
                        {item.label}
                      </Link>
                    );
                  })}

                  {/* Enabled Plugins Section */}
                  {enabledPlugins.length > 0 && (
                    <>
                      <Separator className="my-2" />
                      <p className="px-3 py-2 text-xs font-semibold text-muted-foreground">Plugins</p>
                      {enabledPlugins.map((plugin) => {
                        const isActive = location.pathname === `/plugin/${plugin.id}`;
                        return (
                          <Link
                            key={plugin.id}
                            to={`/plugin/${plugin.id}`}
                            onClick={handleMobileNavClick}
                            className={cn(
                              'group flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all',
                              isActive
                                ? 'bg-primary text-primary-foreground'
                                : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                            )}
                          >
                            {plugin.icon ? (
                              <span className="h-5 w-5 flex items-center justify-center">
                                {plugin.icon}
                              </span>
                            ) : (
                              <Plug2 className="h-5 w-5" />
                            )}
                            <span className="flex-1 truncate">{plugin.name}</span>
                            <Badge
                              variant={plugin.health === 'healthy' ? 'default' : 'destructive'}
                              className={cn(
                                'h-2 w-2 rounded-full p-0',
                                plugin.health === 'healthy' ? 'bg-green-500' : 'bg-red-500'
                              )}
                            />
                          </Link>
                        );
                      })}
                    </>
                  )}
                </nav>

                <Separator className="my-4" />
                
                {/* Logout */}
                <Button 
                  variant="ghost" 
                  className="w-full justify-start text-destructive hover:text-destructive hover:bg-destructive/10"
                  onClick={() => { logout(); setMobileMenuOpen(false); }}
                >
                  <LogOut className="h-5 w-5 mr-3" />
                  Log out
                </Button>
              </div>
            </SheetContent>
          </Sheet>
        </div>
      </header>

      {/* Desktop Sidebar */}
      <aside className="hidden md:fixed md:inset-y-0 md:flex md:w-64 md:flex-col">
        <div className="flex flex-col flex-grow border-r border-border bg-card">
          {/* Logo / Brand */}
          <div className="px-6 py-6">
            <ToruLogo size="md" showText={true} />
          </div>
          
          <Separator />

          {/* Navigation */}
          <nav className="flex-1 space-y-1 px-3 py-4">
            {navItems.map((item) => {
              const Icon = item.icon;
              const isActive = location.pathname === item.path;
              return (
                <Link
                  key={item.path}
                  to={item.path}
                  className={cn(
                    'group flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all',
                    isActive
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                  )}
                >
                  <Icon className="h-5 w-5" />
                  {item.label}
                </Link>
              );
            })}

            {/* Enabled Plugins Section */}
            {enabledPlugins.length > 0 && (
              <>
                <Separator className="my-2" />
                <div className="px-3 py-2">
                  <p className="text-xs font-semibold text-muted-foreground mb-2">Plugins</p>
                  {enabledPlugins.map((plugin) => {
                    const isActive = location.pathname === `/plugin/${plugin.id}`;
                    return (
                      <Link
                        key={plugin.id}
                        to={`/plugin/${plugin.id}`}
                        className={cn(
                          'group flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-all',
                          isActive
                            ? 'bg-primary text-primary-foreground shadow-sm'
                            : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                        )}
                      >
                        {plugin.icon ? (
                          <span className="h-5 w-5 flex items-center justify-center">
                            {plugin.icon}
                          </span>
                        ) : (
                          <Plug2 className="h-5 w-5" />
                        )}
                        <span className="flex-1 truncate">{plugin.name}</span>
                        <Badge
                          variant={plugin.health === 'healthy' ? 'default' : 'destructive'}
                          className={cn(
                            'h-2 w-2 rounded-full p-0',
                            plugin.health === 'healthy' ? 'bg-green-500' : 'bg-red-500'
                          )}
                        />
                      </Link>
                    );
                  })}
                </div>
              </>
            )}
          </nav>

          {/* User Footer */}
          <div className="p-4 border-t border-border">
            <div className="flex items-center gap-3 w-full p-2 rounded-lg bg-accent/50">
              <div className="h-8 w-8 rounded-full bg-primary/20 flex items-center justify-center text-primary">
                <User className="h-4 w-4" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{user?.display_name || user?.username}</p>
                <p className="text-xs text-muted-foreground truncate capitalize">{user?.role}</p>
              </div>
              <Button variant="ghost" size="icon" onClick={() => logout()} title="Logout">
                <LogOut className="h-4 w-4 text-muted-foreground hover:text-destructive" />
              </Button>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <div className="md:pl-64">
        <main className="min-h-screen pt-14 md:pt-0">
          <div className="container py-6 lg:py-8">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}

