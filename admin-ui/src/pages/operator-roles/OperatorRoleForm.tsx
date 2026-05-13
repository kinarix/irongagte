import { useState } from 'react'
import { useMutation, useQueryClient, useQuery } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import * as Dialog from '@radix-ui/react-dialog'
import { createOperatorRole, assignPermissionToRole } from '@/api/operatorRoles'
import { listOperatorPermissions } from '@/api/operatorPermissions'
import { listTenants } from '@/api/tenants'
import { Button } from '@/components/ui/button'
import { ArrowLeft } from 'lucide-react'
import { PermissionSelector, groupSelectedForConfirm } from './PermissionSelector'

const GLOBAL_SCOPE = '__global__'

export default function OperatorRoleForm() {
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [tenantId, setTenantId] = useState<string>(GLOBAL_SCOPE)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [confirmOpen, setConfirmOpen] = useState(false)

  const { data: catalog } = useQuery({
    queryKey: ['operator-permissions'],
    queryFn: listOperatorPermissions,
  })

  const { data: tenantsData } = useQuery({
    queryKey: ['tenants'],
    queryFn: () => listTenants(),
  })

  const createMut = useMutation({
    mutationFn: async () => {
      const role = await createOperatorRole({
        name,
        description: description || undefined,
        tenant_id: tenantId === GLOBAL_SCOPE ? undefined : tenantId,
      })
      await Promise.all(
        [...selected].map((permId) => assignPermissionToRole(role.id, permId)),
      )
      return role
    },
    onSuccess: (role) => {
      void qc.invalidateQueries({ queryKey: ['operator-roles'] })
      void navigate({ to: `/operator-roles/${role.id}` })
    },
  })

  const selectedGrouped = groupSelectedForConfirm(selected, catalog?.permissions ?? [])
  const canSubmit = name.trim().length > 0

  return (
    <div className="max-w-3xl">
      <button
        onClick={() => void navigate({ to: '/operator-roles' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Operator Roles
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-6">New Operator Role</h1>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-4 space-y-3">
        <div>
          <label className="block text-xs text-gray-500 mb-1">Name *</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g. tenant_admin"
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
          />
          <p className="text-xs text-gray-400 mt-1">
            Name is immutable after creation.
          </p>
        </div>
        <div>
          <label className="block text-xs text-gray-500 mb-1">Description</label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
          />
        </div>
        <div>
          <label className="block text-xs text-gray-500 mb-1">Scope</label>
          <select
            value={tenantId}
            onChange={(e) => setTenantId(e.target.value)}
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400 bg-white"
          >
            <option value={GLOBAL_SCOPE}>Global — applies across every tenant</option>
            {tenantsData?.tenants.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name} ({t.slug})
              </option>
            ))}
          </select>
          <p className="text-xs text-gray-400 mt-1">
            A global role grants permissions everywhere. A tenant-scoped role only
            applies to actions inside that tenant. Scope is immutable after creation.
          </p>
        </div>
      </div>

      <PermissionSelector
        selected={selected}
        onChange={setSelected}
        disabled={createMut.isPending}
      />

      <div className="sticky bottom-0 bg-gray-50 -mx-6 px-6 py-3 mt-4 border-t border-gray-200 flex items-center gap-3">
        <Button
          onClick={() => setConfirmOpen(true)}
          disabled={!canSubmit || createMut.isPending}
        >
          Create Role
        </Button>
        <Button
          variant="outline"
          onClick={() => void navigate({ to: '/operator-roles' })}
          disabled={createMut.isPending}
        >
          Cancel
        </Button>
        <span className="text-xs text-gray-500">{selected.size} permission(s)</span>
      </div>

      <Dialog.Root open={confirmOpen} onOpenChange={setConfirmOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/40" />
          <Dialog.Content className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-white rounded-lg shadow-lg p-6 w-[32rem] max-h-[80vh] flex flex-col">
            <Dialog.Title className="text-lg font-semibold mb-1">
              Confirm New Role
            </Dialog.Title>
            <Dialog.Description className="text-sm text-gray-600 mb-4">
              Create role <span className="font-medium">{name || '—'}</span> with{' '}
              {selected.size} permission{selected.size === 1 ? '' : 's'}?
            </Dialog.Description>
            <div className="flex-1 overflow-y-auto border border-gray-200 rounded-md p-3 mb-4 bg-gray-50">
              {selectedGrouped.length === 0 ? (
                <p className="text-sm text-gray-500">No permissions selected.</p>
              ) : (
                <div className="space-y-3">
                  {selectedGrouped.map(([resource, perms]) => (
                    <div key={resource}>
                      <div className="text-xs font-semibold uppercase tracking-wider text-gray-500 mb-1">
                        {resource}
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {perms
                          .slice()
                          .sort((a, b) => a.action.localeCompare(b.action))
                          .map((p) => (
                            <span
                              key={p.id}
                              className="px-2 py-0.5 rounded bg-white border border-gray-200 text-xs font-mono text-gray-700"
                            >
                              {p.action}
                            </span>
                          ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
            <div className="flex justify-end gap-3">
              <Button variant="outline" onClick={() => setConfirmOpen(false)}>
                Cancel
              </Button>
              <Button onClick={() => createMut.mutate()} disabled={createMut.isPending}>
                {createMut.isPending ? 'Creating…' : 'Confirm & Create'}
              </Button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  )
}
