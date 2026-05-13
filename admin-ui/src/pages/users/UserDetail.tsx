import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { getUser, getUserRoles, assignRole, removeRole } from '@/api/users'
import { listRoles } from '@/api/roles'
import { ArrowLeft } from 'lucide-react'

export default function UserDetail() {
  const { userId } = useParams({ strict: false }) as { userId: string }
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()

  const { data: user } = useQuery({
    queryKey: ['user', tenantId, userId],
    queryFn: () => getUser(tenantId!, userId),
    enabled: !!tenantId && !!userId,
  })

  const { data: rolesData } = useQuery({
    queryKey: ['user-roles', tenantId, userId],
    queryFn: () => getUserRoles(tenantId!, userId),
    enabled: !!tenantId && !!userId,
  })

  const { data: allRoles } = useQuery({
    queryKey: ['roles', tenantId],
    queryFn: () => listRoles(tenantId!),
    enabled: !!tenantId,
  })

  const assignMut = useMutation({
    mutationFn: (roleId: string) => assignRole(tenantId!, userId, roleId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['user-roles', tenantId, userId] }),
  })

  const removeMut = useMutation({
    mutationFn: (roleId: string) => removeRole(tenantId!, userId, roleId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['user-roles', tenantId, userId] }),
  })

  const assignedIds = new Set(rolesData?.roles.map((r) => r.id))
  const unassigned = allRoles?.roles.filter((r) => !assignedIds.has(r.id)) ?? []

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>
  if (!user) return <p className="text-sm text-gray-500">Loading…</p>

  return (
    <div className="max-w-2xl">
      <button
        onClick={() => void navigate({ to: '/users' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Users
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-1">{user.email}</h1>
      <p className="text-sm text-gray-500 mb-6">ID: {user.id}</p>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 grid grid-cols-2 gap-3 text-sm">
        <div>
          <span className="text-gray-500">Name</span>
          <p className="font-medium">{user.name ?? '—'}</p>
        </div>
        <div>
          <span className="text-gray-500">Status</span>
          <p className="font-medium capitalize">{user.status}</p>
        </div>
        <div>
          <span className="text-gray-500">Email verified</span>
          <p className="font-medium">{user.email_verified ? 'Yes' : 'No'}</p>
        </div>
        <div>
          <span className="text-gray-500">Created</span>
          <p className="font-medium">{new Date(user.created_at).toLocaleDateString()}</p>
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-medium">Roles</h2>
          {unassigned.length > 0 && (
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) assignMut.mutate(e.target.value)
                e.target.value = ''
              }}
              className="text-sm border border-gray-300 rounded-md px-2 py-1"
            >
              <option value="">Assign role…</option>
              {unassigned.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
          )}
        </div>
        {rolesData?.roles.length ? (
          <ul className="space-y-1">
            {rolesData.roles.map((r) => (
              <li key={r.id} className="flex items-center justify-between py-1.5">
                <span className="text-sm">{r.name}</span>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => removeMut.mutate(r.id)}
                  disabled={removeMut.isPending}
                  className="text-red-600 hover:text-red-700"
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-gray-500">No roles assigned.</p>
        )}
      </div>
    </div>
  )
}
