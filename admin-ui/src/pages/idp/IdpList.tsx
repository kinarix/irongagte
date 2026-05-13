import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listIdpConfigs, deleteIdpConfig, type IdpConfig } from '@/api/idp'

export default function IdpList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [deleteTarget, setDeleteTarget] = useState<IdpConfig | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['idp-configs', tenantId],
    queryFn: () => listIdpConfigs(tenantId!),
    enabled: !!tenantId,
  })

  const deleteMut = useMutation({
    mutationFn: (cfg: IdpConfig) => deleteIdpConfig(tenantId!, cfg.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['idp-configs', tenantId] })
      setDeleteTarget(null)
    },
  })

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant to view identity providers.</p>

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Identity Providers</h1>
        <Button onClick={() => void navigate({ to: '/idp/new' })}>New Provider</Button>
      </div>

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Name</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Type</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Status</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Created</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.configs.map((cfg) => (
                <tr key={cfg.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5 font-medium">{cfg.name}</td>
                  <td className="px-4 py-2.5 text-gray-600 uppercase text-xs">{cfg.provider_type}</td>
                  <td className="px-4 py-2.5">
                    <span
                      className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium ${
                        cfg.enabled ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'
                      }`}
                    >
                      {cfg.enabled ? 'Enabled' : 'Disabled'}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {new Date(cfg.created_at).toLocaleDateString()}
                  </td>
                  <td className="px-4 py-2.5">
                    <div className="flex items-center gap-2 justify-end">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() =>
                          void navigate({ to: '/idp/$idpId/edit', params: { idpId: cfg.id } })
                        }
                      >
                        Edit
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setDeleteTarget(cfg)}
                        className="text-red-600 hover:text-red-700"
                      >
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
              {!data?.configs.length && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-sm text-gray-500">
                    No identity providers configured.
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
        title="Delete Identity Provider"
        description={`Remove "${deleteTarget?.name}"? Users who signed in via this provider will lose that login method.`}
        confirmLabel="Remove"
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
