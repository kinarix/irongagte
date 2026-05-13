import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listTenants, createTenant, deleteTenant, type Tenant } from '@/api/tenants'

export default function TenantList() {
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [name, setName] = useState('')
  const [slug, setSlug] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<Tenant | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['tenants'],
    queryFn: () => listTenants(),
  })

  const createMut = useMutation({
    mutationFn: () => createTenant({ name, slug }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['tenants'] })
      setShowCreate(false)
      setName('')
      setSlug('')
    },
  })

  const deleteMut = useMutation({
    mutationFn: (t: Tenant) => deleteTenant(t.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['tenants'] })
      setDeleteTarget(null)
    },
  })

  const navigate = useNavigate()

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Tenants</h1>
        <Button onClick={() => setShowCreate(true)}>New Tenant</Button>
      </div>

      <p className="text-sm text-gray-500 mb-4">
        Tenants isolate users, applications, roles and permissions. Creating a tenant
        also seeds the full permission catalog for it.
      </p>

      {showCreate && (
        <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 max-w-md">
          <h2 className="font-medium mb-3">Create Tenant</h2>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Name *</label>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">Slug *</label>
              <input
                value={slug}
                onChange={(e) => setSlug(e.target.value)}
                placeholder="acme-corp"
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMut.mutate()}
                disabled={!name || !slug || createMut.isPending}
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
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Slug</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">ID</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.tenants.map((t) => (
                <tr
                  key={t.id}
                  className="border-b border-gray-100 hover:bg-gray-50 cursor-pointer"
                  onClick={() =>
                    void navigate({ to: '/tenants/$tenantId', params: { tenantId: t.id } })
                  }
                >
                  <td className="px-4 py-2.5 font-medium">{t.name}</td>
                  <td className="px-4 py-2.5 text-gray-600">{t.slug}</td>
                  <td className="px-4 py-2.5 text-gray-500 font-mono text-xs">{t.id}</td>
                  <td className="px-4 py-2.5">
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={(e) => {
                        e.stopPropagation()
                        setDeleteTarget(t)
                      }}
                      className="text-red-600 hover:text-red-700"
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))}
              {!data?.tenants.length && (
                <tr>
                  <td colSpan={4} className="px-4 py-8 text-center text-sm text-gray-500">
                    No tenants yet.
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
        title="Delete Tenant"
        description={`Soft-delete tenant "${deleteTarget?.name}"? Users and apps under it will no longer be accessible.`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
