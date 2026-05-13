import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ConfirmDialog'
import {
  listOperators,
  createOperator,
  deleteOperator,
  updateOperator,
  changeOperatorPassword,
  type Operator,
} from '@/api/operators'

export default function OperatorList() {
  const qc = useQueryClient()
  const [showCreate, setShowCreate] = useState(false)
  const [email, setEmail] = useState('')
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<Operator | null>(null)
  const [pwTarget, setPwTarget] = useState<Operator | null>(null)
  const [newPassword, setNewPassword] = useState('')

  const { data, isLoading } = useQuery({
    queryKey: ['operators'],
    queryFn: () => listOperators(),
  })

  const createMut = useMutation({
    mutationFn: () =>
      createOperator({ email, name: name || undefined, password }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['operators'] })
      setShowCreate(false)
      setEmail('')
      setName('')
      setPassword('')
    },
  })

  const deleteMut = useMutation({
    mutationFn: (o: Operator) => deleteOperator(o.id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['operators'] })
      setDeleteTarget(null)
    },
  })

  const toggleStatusMut = useMutation({
    mutationFn: (o: Operator) =>
      updateOperator(o.id, {
        status: o.status === 'active' ? 'suspended' : 'active',
      }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['operators'] }),
  })

  const pwMut = useMutation({
    mutationFn: () => changeOperatorPassword(pwTarget!.id, newPassword),
    onSuccess: () => {
      setPwTarget(null)
      setNewPassword('')
    },
  })

  return (
    <div>
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Operators</h1>
        <Button onClick={() => setShowCreate(true)}>New Operator</Button>
      </div>

      <p className="text-sm text-gray-500 mb-4">
        Operators are irongate administrators who manage the system itself — tenants,
        applications, roles, etc. They are not the same as end users authenticating
        through irongate.
      </p>

      {showCreate && (
        <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 max-w-md">
          <h2 className="font-medium mb-3">Create Operator</h2>
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
            <div>
              <label className="block text-xs text-gray-500 mb-1">Password *</label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
              />
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                onClick={() => createMut.mutate()}
                disabled={!email || !password || createMut.isPending}
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
                <th className="text-left px-4 py-2.5 font-medium text-gray-500">Last login</th>
                <th className="px-4 py-2.5" />
              </tr>
            </thead>
            <tbody>
              {data?.operators.map((o) => (
                <tr key={o.id} className="border-b border-gray-100 hover:bg-gray-50">
                  <td className="px-4 py-2.5 font-medium">{o.email}</td>
                  <td className="px-4 py-2.5 text-gray-600">{o.name ?? '—'}</td>
                  <td className="px-4 py-2.5">
                    <span
                      className={
                        o.status === 'active'
                          ? 'text-green-700 text-xs font-medium'
                          : 'text-amber-700 text-xs font-medium'
                      }
                    >
                      {o.status}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-gray-500">
                    {o.last_login_at ? new Date(o.last_login_at).toLocaleString() : '—'}
                  </td>
                  <td className="px-4 py-2.5 text-right">
                    <Button size="sm" variant="ghost" onClick={() => setPwTarget(o)}>
                      Password
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => toggleStatusMut.mutate(o)}>
                      {o.status === 'active' ? 'Suspend' : 'Activate'}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => setDeleteTarget(o)}
                      className="text-red-600 hover:text-red-700"
                    >
                      Delete
                    </Button>
                  </td>
                </tr>
              ))}
              {!data?.operators.length && (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-sm text-gray-500">
                    No operators yet.
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
        title="Delete Operator"
        description={`Permanently delete "${deleteTarget?.email}"?`}
        onConfirm={() => deleteTarget && deleteMut.mutate(deleteTarget)}
        loading={deleteMut.isPending}
      />

      {pwTarget && (
        <div className="fixed inset-0 bg-black/30 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg border border-gray-200 shadow-lg p-6 w-96">
            <h2 className="font-medium mb-1">Change password</h2>
            <p className="text-sm text-gray-500 mb-3">{pwTarget.email}</p>
            <input
              type="password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              placeholder="New password"
              className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
            />
            <div className="flex gap-2 mt-4 justify-end">
              <Button variant="outline" size="sm" onClick={() => { setPwTarget(null); setNewPassword('') }}>
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={() => pwMut.mutate()}
                disabled={!newPassword || pwMut.isPending}
              >
                Update
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
