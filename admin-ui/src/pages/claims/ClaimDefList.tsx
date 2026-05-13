import { useMemo, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import {
  deleteClaimDefinition,
  listClaimDefinitions,
  type ClaimDefinition,
} from '@/api/claims'
import { listApplications, type Application } from '@/api/applications'

export default function ClaimDefList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [appFilter, setAppFilter] = useState<string>('')
  const [deleteTarget, setDeleteTarget] = useState<ClaimDefinition | null>(null)

  const appsQuery = useQuery({
    queryKey: ['applications', tenantId],
    queryFn: () => listApplications(tenantId!),
    enabled: !!tenantId,
  })

  const defsQuery = useQuery({
    queryKey: ['claim-definitions', tenantId, appFilter],
    queryFn: () => listClaimDefinitions(tenantId!, appFilter || undefined),
    enabled: !!tenantId,
  })

  const deleteMut = useMutation({
    mutationFn: (d: ClaimDefinition) => deleteClaimDefinition(tenantId!, d.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['claim-definitions', tenantId] })
      setDeleteTarget(null)
    },
  })

  const appsById = useMemo(() => {
    const map = new Map<string, Application>()
    for (const a of appsQuery.data?.applications ?? []) map.set(a.id, a)
    return map
  }, [appsQuery.data])

  if (!tenantId) {
    return (
      <p className="text-sm text-gray-500">Select a tenant to view claim definitions.</p>
    )
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Claim Definitions</h1>
          <p className="text-sm text-gray-500 mt-1">
            Custom JWT claims per application. Final token key is{' '}
            <code>&lt;app.claim_prefix&gt;:&lt;key&gt;</code>.
          </p>
        </div>
        <Button onClick={() => void navigate({ to: '/claims/new' })}>New Claim</Button>
      </div>

      <div className="mb-4">
        <label className="text-sm text-gray-600 mr-2">Application:</label>
        <select
          value={appFilter}
          onChange={(e) => setAppFilter(e.target.value)}
          className="text-sm border border-gray-300 rounded px-2 py-1"
        >
          <option value="">All</option>
          {appsQuery.data?.applications.map((a) => (
            <option key={a.id} value={a.id}>
              {a.name} ({a.claim_prefix})
            </option>
          ))}
        </select>
      </div>

      {defsQuery.isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">App</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Prefix</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Key</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Type</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Description</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {defsQuery.data?.claim_definitions.map((def) => {
                const app = appsById.get(def.application_id)
                return (
                  <tr key={def.id} className="border-b border-gray-100 hover:bg-gray-50">
                    <td className="px-4 py-2.5">{app?.name ?? def.application_id}</td>
                    <td className="px-4 py-2.5 font-mono text-xs text-gray-600">
                      {app?.claim_prefix ?? '—'}
                    </td>
                    <td className="px-4 py-2.5 font-medium">{def.key}</td>
                    <td className="px-4 py-2.5">
                      <span
                        className={`text-xs px-2 py-0.5 rounded ${
                          def.claim_type === 'multi'
                            ? 'bg-indigo-100 text-indigo-700'
                            : 'bg-gray-100 text-gray-700'
                        }`}
                      >
                        {def.claim_type}
                      </span>
                    </td>
                    <td className="px-4 py-2.5 text-gray-500">{def.description ?? '—'}</td>
                    <td className="px-4 py-2.5">
                      <div className="flex items-center gap-2 justify-end">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() =>
                            void navigate({
                              to: '/claims/$claimId/edit',
                              params: { claimId: def.id },
                            })
                          }
                        >
                          Edit
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => setDeleteTarget(def)}
                          className="text-red-600 hover:text-red-700"
                        >
                          Delete
                        </Button>
                      </div>
                    </td>
                  </tr>
                )
              })}
              {!defsQuery.data?.claim_definitions.length && (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-sm text-gray-500">
                    No claim definitions yet.
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
        title="Delete Claim Definition"
        description={`Delete claim "${deleteTarget?.key}"? Existing group and user assignments for this claim will also be removed.`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
