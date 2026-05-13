import { Outlet, Link, useNavigate } from '@tanstack/react-router'
import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getAccessToken, clearAccessToken } from '@/auth'
import { useTenant } from '@/context/tenant'
import { listTenants } from '@/api/tenants'
import { Users, AppWindow, Shield, Settings, LogOut } from 'lucide-react'

const navItems = [
  { to: '/users' as const, label: 'Users', icon: Users },
  { to: '/applications' as const, label: 'Applications', icon: AppWindow },
  { to: '/roles' as const, label: 'Roles', icon: Shield },
  { to: '/idp' as const, label: 'Identity Providers', icon: Settings },
]

export default function Layout() {
  const navigate = useNavigate()
  const { tenantId, setTenantId } = useTenant()

  useEffect(() => {
    if (!getAccessToken()) {
      void navigate({ to: '/login' })
    }
  }, [navigate])

  const { data } = useQuery({
    queryKey: ['tenants'],
    queryFn: () => listTenants(),
    enabled: !!getAccessToken(),
  })

  function logout() {
    clearAccessToken()
    void navigate({ to: '/login' })
  }

  return (
    <div className="flex h-screen bg-gray-50">
      <aside className="w-56 bg-white border-r border-gray-200 flex flex-col shrink-0">
        <div className="px-4 py-4 border-b border-gray-200">
          <span className="text-base font-semibold text-gray-900">Irongate Admin</span>
        </div>

        <nav className="flex-1 p-3 space-y-0.5">
          {navItems.map(({ to, label, icon: Icon }) => (
            <Link
              key={to}
              to={to}
              activeProps={{ className: 'bg-gray-100 font-medium text-gray-900' }}
              inactiveProps={{ className: 'text-gray-600 hover:bg-gray-50 hover:text-gray-900' }}
              className="flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors"
            >
              <Icon size={15} />
              {label}
            </Link>
          ))}
        </nav>

        <div className="p-3 border-t border-gray-200">
          <button
            onClick={logout}
            className="flex items-center gap-3 px-3 py-2 w-full rounded-md text-sm text-gray-600 hover:bg-gray-50 hover:text-gray-900 transition-colors"
          >
            <LogOut size={15} />
            Sign out
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col overflow-hidden">
        <header className="bg-white border-b border-gray-200 px-6 h-12 flex items-center gap-3">
          <span className="text-sm text-gray-500 ml-auto">Tenant</span>
          <select
            value={tenantId ?? ''}
            onChange={(e) => setTenantId(e.target.value)}
            className="text-sm border border-gray-300 rounded-md px-2 py-1 bg-white focus:outline-none focus:ring-2 focus:ring-gray-400"
          >
            <option value="">— select —</option>
            {data?.tenants.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
          </select>
        </header>

        <main className="flex-1 overflow-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
