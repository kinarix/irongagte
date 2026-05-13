import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listRoles, createRole, deleteRole, type Role } from '@/api/roles'

export default function RoleList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [roleName, setRoleName] = useState('')
  const [description, setDescription] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<Role | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['roles', tenantId],
    queryFn: () => listRoles(tenantId!),
    enabled: !!tenantId,
  })

  const createMut = useMutation({
    mutationFn: () =>
      createRole({
        tenant_id: tenantId!,
        name: roleName,
        description: description || undefined,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['roles', tenantId] })
      setShowCreate(false)
      setRoleName('')
      setDescription('')
    },
  })

  const deleteMut = useMutation({
    mutationFn: (r: Role) => deleteRole(tenantId!, r.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['roles', tenantId] })
      setDeleteTarget(null)
    },
  })

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant to view roles.</p>

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Roles</h1>
        <Button onClick={() => setShowCreate(true)}>New Role</Button>
      </div>

      {showCreate && (
        <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 max-w-md">
          <h2 className="font-medium mb-3">Create Role</h2>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Name *</label>
              <input
                value={roleName}
                onChange={(e) => setRoleName(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
                placeholder="e.g. editor"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">Description</label>
              <input
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMut.mutate()}
                disabled={!roleName || createMut.isPending}
              >
                Create
              </Button>
              <Button size="sm" variant="outline" onClick={() => setShowCreate(false)}>
                Cancel
              </Button>
            </div>
          </div>
        </div>
      )}

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Name</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Description</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Created</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.roles.map((r) => (
                <tr key={r.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5">
                    <button
                      onClick={() =>
                        void navigate({ to: '/roles/$roleId', params: { roleId: r.id } })
                      }
                      className="text-blue-600 hover:underline font-medium"
                    >
                      {r.name}
                    </button>
                  </td>
                  <td className="px-4 py-2.5 text-gray-600">{r.description ?? '—'}</td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {new Date(r.created_at).toLocaleDateString()}
                  </td>
                  <td className="px-4 py-2.5">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setDeleteTarget(r)}
                      className="text-red-600 hover:text-red-700"
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))}
              {!data?.roles.length && (
                <tr>
                  <td colSpan={4} className="px-4 py-8 text-center text-sm text-gray-500">
                    No roles yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      <ConfirmDialog
        open={!!deleteTarget}
        onOpenChange={(o) => !o && setDeleteTarget(null)}
        title="Delete Role"
        description={`Permanently delete role "${deleteTarget?.name}"? This cannot be undone.`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
