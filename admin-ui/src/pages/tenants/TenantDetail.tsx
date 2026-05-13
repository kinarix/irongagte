import { useQuery } from '@tanstack/react-query'
import { useNavigate, useParams } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { useTenant } from '@/context/tenant'
import { getTenant } from '@/api/tenants'
import { listUsers } from '@/api/users'
import { listApplications } from '@/api/applications'
import { listGroups } from '@/api/groups'
import { listIdpConfigs } from '@/api/idp'
import { listClaimDefinitions } from '@/api/claims'
import {
  AppWindow,
  ArrowLeft,
  ArrowRight,
  KeyRound,
  Tag,
  Users,
  UsersRound,
} from 'lucide-react'

export default function TenantDetail() {
  const { tenantId } = useParams({ strict: false }) as { tenantId: string }
  const navigate = useNavigate()
  const { setTenantId } = useTenant()

  const tenantQuery = useQuery({
    queryKey: ['tenant', tenantId],
    queryFn: () => getTenant(tenantId),
    enabled: !!tenantId,
  })

  const usersQuery = useQuery({
    queryKey: ['users', tenantId, 'count'],
    queryFn: () => listUsers(tenantId, 1, 0),
    enabled: !!tenantId,
  })
  const appsQuery = useQuery({
    queryKey: ['applications', tenantId, 'count'],
    queryFn: () => listApplications(tenantId, 1, 0),
    enabled: !!tenantId,
  })
  const groupsQuery = useQuery({
    queryKey: ['groups', tenantId, 'count'],
    queryFn: () => listGroups(tenantId, 1, 0),
    enabled: !!tenantId,
  })
  const idpsQuery = useQuery({
    queryKey: ['idp-configs', tenantId, 'count'],
    queryFn: () => listIdpConfigs(tenantId),
    enabled: !!tenantId,
  })
  const claimsQuery = useQuery({
    queryKey: ['claim-definitions', tenantId, 'count'],
    queryFn: () => listClaimDefinitions(tenantId),
    enabled: !!tenantId,
  })

  function goWithTenant(to: '/users' | '/applications' | '/groups' | '/idp' | '/claims') {
    setTenantId(tenantId)
    void navigate({ to })
  }

  if (!tenantQuery.data) {
    return <p className="text-sm text-gray-500">Loading…</p>
  }

  const t = tenantQuery.data
  const counters: Array<{
    label: string
    count: number | undefined
    icon: typeof Users
    to: '/users' | '/applications' | '/groups' | '/idp' | '/claims'
  }> = [
    { label: 'Users', count: usersQuery.data?.total, icon: Users, to: '/users' },
    { label: 'Applications', count: appsQuery.data?.total, icon: AppWindow, to: '/applications' },
    { label: 'Groups', count: groupsQuery.data?.total, icon: UsersRound, to: '/groups' },
    {
      label: 'IdP configs',
      count: idpsQuery.data?.configs.length,
      icon: KeyRound,
      to: '/idp',
    },
    {
      label: 'Claim defs',
      count: claimsQuery.data?.total,
      icon: Tag,
      to: '/claims',
    },
  ]

  return (
    <div className="max-w-4xl">
      <button
        onClick={() => void navigate({ to: '/tenants' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Tenants
      </button>

      <div className="flex items-start justify-between mb-6">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">{t.name}</h1>
          <p className="text-sm text-gray-500 mt-1">
            Slug: <span className="font-mono">{t.slug}</span> · ID:{' '}
            <span className="font-mono text-xs">{t.id}</span> · Created{' '}
            {new Date(t.created_at).toLocaleDateString()}
          </p>
        </div>
        <Button
          variant="outline"
          onClick={() => {
            setTenantId(tenantId)
            void navigate({ to: '/users' })
          }}
        >
          Enter Tenant
        </Button>
      </div>

      <div className="grid grid-cols-5 gap-3 mb-8">
        {counters.map((c) => {
          const Icon = c.icon
          return (
            <button
              key={c.label}
              onClick={() => goWithTenant(c.to)}
              className="bg-white border border-gray-200 rounded-lg px-4 py-3 text-left hover:border-gray-400 hover:shadow-sm transition"
            >
              <Icon size={16} className="text-gray-400 mb-1.5" />
              <div className="text-2xl font-semibold text-gray-900">
                {c.count ?? '—'}
              </div>
              <div className="text-xs text-gray-500">{c.label}</div>
            </button>
          )
        })}
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-5">
        <h2 className="font-medium text-gray-900 mb-3">Quick links</h2>
        <div className="grid grid-cols-2 gap-y-1.5 text-sm">
          <QuickLink icon={Users} label="Users" onClick={() => goWithTenant('/users')} />
          <QuickLink
            icon={AppWindow}
            label="Applications"
            onClick={() => goWithTenant('/applications')}
          />
          <QuickLink icon={Tag} label="Claims" onClick={() => goWithTenant('/claims')} />
          <QuickLink
            icon={UsersRound}
            label="Groups"
            onClick={() => goWithTenant('/groups')}
          />
          <QuickLink
            icon={KeyRound}
            label="Identity providers"
            onClick={() => goWithTenant('/idp')}
          />
        </div>
      </div>
    </div>
  )
}

function QuickLink({
  icon: Icon,
  label,
  onClick,
}: {
  icon: typeof Users
  label: string
  onClick: () => void
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 text-gray-700 hover:text-gray-900 py-1 group"
    >
      <Icon size={14} className="text-gray-400 group-hover:text-gray-600" />
      <span>{label}</span>
      <ArrowRight
        size={12}
        className="text-gray-300 group-hover:text-gray-500 group-hover:translate-x-0.5 transition"
      />
    </button>
  )
}
