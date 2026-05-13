import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listGroups, createGroup, deleteGroup, type Group } from '@/api/groups'

export default function GroupList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [displayName, setDisplayName] = useState('')
  const [externalId, setExternalId] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<Group | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['groups', tenantId],
    queryFn: () => listGroups(tenantId!),
    enabled: !!tenantId,
  })

  const createMut = useMutation({
    mutationFn: () =>
      createGroup({
        tenant_id: tenantId!,
        display_name: displayName,
        external_id: externalId || undefined,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['groups', tenantId] })
      setShowCreate(false)
      setDisplayName('')
      setExternalId('')
    },
  })

  const deleteMut = useMutation({
    mutationFn: (g: Group) => deleteGroup(tenantId!, g.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['groups', tenantId] })
      setDeleteTarget(null)
    },
  })

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant to view groups.</p>

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Groups</h1>
        <Button onClick={() => setShowCreate(true)}>New Group</Button>
      </div>

      {showCreate && (
        <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 max-w-md">
          <h2 className="font-medium mb-3">Create Group</h2>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Display name *</label>
              <input
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
                placeholder="e.g. Engineering"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">External ID</label>
              <input
                value={externalId}
                onChange={(e) => setExternalId(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
                placeholder="optional"
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMut.mutate()}
                disabled={!displayName || createMut.isPending}
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
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">External ID</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Created</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.groups.map((g) => (
                <tr key={g.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5">
                    <button
                      onClick={() =>
                        void navigate({ to: '/groups/$groupId', params: { groupId: g.id } })
                      }
                      className="text-blue-600 hover:underline font-medium"
                    >
                      {g.display_name}
                    </button>
                  </td>
                  <td className="px-4 py-2.5 text-gray-600">{g.external_id ?? '—'}</td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {new Date(g.created_at).toLocaleDateString()}
                  </td>
                  <td className="px-4 py-2.5">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setDeleteTarget(g)}
                      className="text-red-600 hover:text-red-700"
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))}
              {!data?.groups.length && (
                <tr>
                  <td colSpan={4} className="px-4 py-8 text-center text-sm text-gray-500">
                    No groups yet.
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
        title="Delete Group"
        description={`Permanently delete "${deleteTarget?.display_name}"?`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
