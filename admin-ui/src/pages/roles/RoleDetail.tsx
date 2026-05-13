import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import {
  getRole,
  listRolePermissions,
  assignPermission,
  removePermission,
  listPermissions,
} from '@/api/roles'
import { ArrowLeft } from 'lucide-react'

export default function RoleDetail() {
  const { roleId } = useParams({ strict: false }) as { roleId: string }
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()

  const { data: role } = useQuery({
    queryKey: ['role', tenantId, roleId],
    queryFn: () => getRole(tenantId!, roleId),
    enabled: !!tenantId && !!roleId,
  })

  const { data: rolePerms } = useQuery({
    queryKey: ['role-permissions', tenantId, roleId],
    queryFn: () => listRolePermissions(tenantId!, roleId),
    enabled: !!tenantId && !!roleId,
  })

  const { data: allPerms } = useQuery({
    queryKey: ['permissions', tenantId],
    queryFn: () => listPermissions(tenantId!),
    enabled: !!tenantId,
  })

  const assignMut = useMutation({
    mutationFn: (permId: string) => assignPermission(tenantId!, roleId, permId),
    onSuccess: () =>
      void qc.invalidateQueries({ queryKey: ['role-permissions', tenantId, roleId] }),
  })

  const removeMut = useMutation({
    mutationFn: (permId: string) => removePermission(tenantId!, roleId, permId),
    onSuccess: () =>
      void qc.invalidateQueries({ queryKey: ['role-permissions', tenantId, roleId] }),
  })

  const assignedIds = new Set(rolePerms?.permissions.map((p) => p.id))
  const unassigned = allPerms?.permissions.filter((p) => !assignedIds.has(p.id)) ?? []

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>
  if (!role) return <p className="text-sm text-gray-500">Loading…</p>

  return (
    <div className="max-w-2xl">
      <button
        onClick={() => void navigate({ to: '/roles' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Roles
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-1">{role.name}</h1>
      {role.description && <p className="text-sm text-gray-500 mb-1">{role.description}</p>}
      <p className="text-xs text-gray-400 mb-6">ID: {role.id}</p>

      <div className="bg-white border border-gray-200 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-medium">Permissions</h2>
          {unassigned.length > 0 && (
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) assignMut.mutate(e.target.value)
                e.target.value = ''
              }}
              className="text-sm border border-gray-300 rounded-md px-2 py-1"
            >
              <option value="">Assign permission…</option>
              {unassigned.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.resource}:{p.action}
                </option>
              ))}
            </select>
          )}
        </div>
        {rolePerms?.permissions.length ? (
          <ul className="space-y-1">
            {rolePerms.permissions.map((p) => (
              <li key={p.id} className="flex items-center justify-between py-1.5">
                <div>
                  <span className="text-sm font-mono">
                    {p.resource}:{p.action}
                  </span>
                  {p.description && (
                    <span className="text-xs text-gray-500 ml-2">{p.description}</span>
                  )}
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => removeMut.mutate(p.id)}
                  disabled={removeMut.isPending}
                  className="text-red-600 hover:text-red-700"
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-gray-500">No permissions assigned.</p>
        )}
      </div>
    </div>
  )
}
