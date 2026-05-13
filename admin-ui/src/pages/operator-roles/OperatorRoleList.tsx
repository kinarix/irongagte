import { useState, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import {
  listOperatorRoles,
  deleteOperatorRole,
  type OperatorRole,
} from '@/api/operatorRoles'
import { listTenants } from '@/api/tenants'
import { Trash2 } from 'lucide-react'

export default function OperatorRoleList() {
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [deleteTarget, setDeleteTarget] = useState<OperatorRole | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['operator-roles'],
    queryFn: () => listOperatorRoles(),
  })

  const { data: tenantsData } = useQuery({
    queryKey: ['tenants'],
    queryFn: () => listTenants(),
  })

  const tenantName = useMemo(() => {
    const m = new Map<string, string>()
    for (const t of tenantsData?.tenants ?? []) m.set(t.id, t.name)
    return (id: string | null) => (id === null ? null : m.get(id) ?? id)
  }, [tenantsData])

  const deleteMut = useMutation({
    mutationFn: (r: OperatorRole) => deleteOperatorRole(r.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['operator-roles'] })
      setDeleteTarget(null)
    },
  })

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Operator Roles</h1>
        <Button onClick={() => void navigate({ to: '/operator-roles/new' })}>
          New Operator Role
        </Button>
      </div>

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Name</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Scope</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Description</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.roles.map((r) => (
                <tr key={r.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5">
                    <button
                      onClick={() => navigate({ to: `/operator-roles/${r.id}` })}
                      className="font-medium text-blue-600 hover:text-blue-700"
                    >
                      {r.name}
                    </button>
                  </td>
                  <td className="px-4 py-2.5">
                    {r.tenant_id === null ? (
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-purple-50 text-purple-700 border border-purple-200">
                        Global
                      </span>
                    ) : (
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-50 text-blue-700 border border-blue-200">
                        {tenantName(r.tenant_id)}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-2.5 text-gray-600">{r.description ?? '—'}</td>
                  <td className="px-4 py-2.5">
                    <button
                      onClick={() => setDeleteTarget(r)}
                      className="text-gray-400 hover:text-red-600"
                    >
                      <Trash2 size={16} />
                    </button>
                  </td>
                </tr>
              ))}
              {!data?.roles.length && (
                <tr>
                  <td colSpan={4} className="px-4 py-8 text-center text-sm text-gray-500">
                    No operator roles yet.
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
        title="Delete Operator Role"
        description={`Permanently delete "${deleteTarget?.name}"?`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
