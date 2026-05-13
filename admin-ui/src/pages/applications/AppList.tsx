import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listApplications, deleteApplication, type Application } from '@/api/applications'

export default function AppList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [deleteTarget, setDeleteTarget] = useState<Application | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['applications', tenantId],
    queryFn: () => listApplications(tenantId!),
    enabled: !!tenantId,
  })

  const deleteMut = useMutation({
    mutationFn: (app: Application) => deleteApplication(tenantId!, app.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['applications', tenantId] })
      setDeleteTarget(null)
    },
  })

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant to view applications.</p>

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Applications</h1>
        <Button onClick={() => void navigate({ to: '/applications/new' })}>New Application</Button>
      </div>

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="bg-white border border-gray-200 rounded-lg overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50">
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Name</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Client ID</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Type</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Created</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.applications.map((app) => (
                <tr key={app.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5 font-medium">{app.name}</td>
                  <td className="px-4 py-2.5 font-mono text-xs text-gray-600">{app.client_id}</td>
                  <td className="px-4 py-2.5 text-gray-600 capitalize">{app.app_type}</td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {new Date(app.created_at).toLocaleDateString()}
                  </td>
                  <td className="px-4 py-2.5">
                    <div className="flex items-center gap-2 justify-end">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() =>
                          void navigate({
                            to: '/applications/$appId/edit',
                            params: { appId: app.id },
                          })
                        }
                      >
                        Edit
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setDeleteTarget(app)}
                        className="text-red-600 hover:text-red-700"
                      >
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
              {!data?.applications.length && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-sm text-gray-500">
                    No applications yet.
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
        title="Delete Application"
        description={`Permanently delete "${deleteTarget?.name}"? This cannot be undone.`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
