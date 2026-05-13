import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import { listUsers, createUser, deleteUser, suspendUser, unsuspendUser, type User } from '@/api/users'

export default function UserList() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [email, setEmail] = useState('')
  const [name, setName] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<User | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['users', tenantId],
    queryFn: () => listUsers(tenantId!),
    enabled: !!tenantId,
  })

  const createMut = useMutation({
    mutationFn: () => createUser({ tenant_id: tenantId!, email, name: name || undefined }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['users', tenantId] })
      setShowCreate(false)
      setEmail('')
      setName('')
    },
  })

  const deleteMut = useMutation({
    mutationFn: (u: User) => deleteUser(tenantId!, u.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['users', tenantId] })
      setDeleteTarget(null)
    },
  })

  const suspendMut = useMutation({
    mutationFn: (u: User) =>
      u.status === 'suspended' ? unsuspendUser(tenantId!, u.id) : suspendUser(tenantId!, u.id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['users', tenantId] }),
  })

  if (!tenantId) {
    return <p className="text-sm text-gray-500">Select a tenant to view users.</p>
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Users</h1>
        <Button onClick={() => setShowCreate(true)}>New User</Button>
      </div>

      {showCreate && (
        <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 max-w-md">
          <h2 className="font-medium mb-3">Create User</h2>
          <div className="space-y-3">
            <div>
              <label className="block text-xs text-gray-500 mb-1">Email *</label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div>
              <label className="block text-xs text-gray-500 mb-1">Name</label>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMut.mutate()}
                disabled={!email || createMut.isPending}
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
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Email</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Name</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Status</th>
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Last Login</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.users.map((u) => (
                <tr key={u.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5">
                    <button
                      onClick={() => void navigate({ to: '/users/$userId', params: { userId: u.id } })}
                      className="text-blue-600 hover:underline"
                    >
                      {u.email}
                    </button>
                  </td>
                  <td className="px-4 py-2.5 text-gray-600">{u.name ?? '—'}</td>
                  <td className="px-4 py-2.5">
                    <span
                      className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium ${
                        u.status === 'active'
                          ? 'bg-green-100 text-green-700'
                          : 'bg-red-100 text-red-700'
                      }`}
                    >
                      {u.status}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {u.last_login_at ? new Date(u.last_login_at).toLocaleDateString() : '—'}
                  </td>
                  <td className="px-4 py-2.5">
                    <div className="flex items-center gap-2 justify-end">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => suspendMut.mutate(u)}
                        disabled={suspendMut.isPending}
                      >
                        {u.status === 'suspended' ? 'Unsuspend' : 'Suspend'}
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setDeleteTarget(u)}
                        className="text-red-600 hover:text-red-700"
                      >
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
              {!data?.users.length && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-sm text-gray-500">
                    No users yet.
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
        title="Delete User"
        description={`Permanently delete ${deleteTarget?.email}? This cannot be undone.`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />
    </div>
  )
}
