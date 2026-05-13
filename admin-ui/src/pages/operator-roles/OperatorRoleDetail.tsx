import { useEffect, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import * as Dialog from '@radix-ui/react-dialog'
import {
  getOperatorRole,
  listRolePermissions,
  assignPermissionToRole,
  revokePermissionFromRole,
  updateOperatorRole,
} from '@/api/operatorRoles'
import { listOperatorPermissions } from '@/api/operatorPermissions'
import { listTenants } from '@/api/tenants'
import { Button } from '@/components/ui/button'
import { ArrowLeft } from 'lucide-react'
import { PermissionSelector, groupSelectedForConfirm } from './PermissionSelector'

export default function OperatorRoleDetail() {
  const { roleId } = useParams({ strict: false }) as { roleId: string }
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [initial, setInitial] = useState<Set<string>>(new Set())
  const [description, setDescription] = useState('')
  const [initialDescription, setInitialDescription] = useState('')
  const [confirmOpen, setConfirmOpen] = useState(false)

  const { data: role } = useQuery({
    queryKey: ['operator-role', roleId],
    queryFn: () => getOperatorRole(roleId),
    enabled: !!roleId,
  })

  const { data: rolePermsData } = useQuery({
    queryKey: ['operator-role-permissions', roleId],
    queryFn: () => listRolePermissions(roleId),
    enabled: !!roleId,
  })

  const { data: catalog } = useQuery({
    queryKey: ['operator-permissions'],
    queryFn: listOperatorPermissions,
  })

  const { data: tenantsData } = useQuery({
    queryKey: ['tenants'],
    queryFn: () => listTenants(),
    enabled: !!role?.tenant_id,
  })

  const scopeLabel = (() => {
    if (!role) return null
    if (role.tenant_id === null) return 'Global'
    const t = tenantsData?.tenants.find((t) => t.id === role.tenant_id)
    return t ? `${t.name} (${t.slug})` : role.tenant_id
  })()

  useEffect(() => {
    if (rolePermsData) {
      const ids = new Set(rolePermsData.permissions.map((p) => p.id))
      setSelected(new Set(ids))
      setInitial(new Set(ids))
    }
  }, [rolePermsData])

  useEffect(() => {
    if (role) {
      setDescription(role.description ?? '')
      setInitialDescription(role.description ?? '')
    }
  }, [role])

  const saveMut = useMutation({
    mutationFn: async () => {
      if (description !== initialDescription) {
        await updateOperatorRole(roleId, { description })
      }
      const toAdd = [...selected].filter((id) => !initial.has(id))
      const toRemove = [...initial].filter((id) => !selected.has(id))
      await Promise.all([
        ...toAdd.map((id) => assignPermissionToRole(roleId, id)),
        ...toRemove.map((id) => revokePermissionFromRole(roleId, id)),
      ])
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['operator-role-permissions', roleId] })
      void qc.invalidateQueries({ queryKey: ['operator-role', roleId] })
      void qc.invalidateQueries({ queryKey: ['operator-roles'] })
      setConfirmOpen(false)
    },
  })

  if (!role) return <p className="text-sm text-gray-500">Loading…</p>

  const dirty =
    description !== initialDescription ||
    selected.size !== initial.size ||
    [...selected].some((id) => !initial.has(id))

  const reset = () => {
    setSelected(new Set(initial))
    setDescription(initialDescription)
  }

  const selectedGrouped = groupSelectedForConfirm(selected, catalog?.permissions ?? [])

  return (
    <div className="max-w-3xl">
      <button
        onClick={() => void navigate({ to: '/operator-roles' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Operator Roles
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-6">Edit Operator Role</h1>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-4 space-y-3">
        <div>
          <label className="block text-xs text-gray-500 mb-1">Name</label>
          <div className="w-full border border-gray-200 bg-gray-50 rounded-md px-3 py-1.5 text-sm text-gray-700 font-mono">
            {role.name}
          </div>
          <p className="text-xs text-gray-400 mt-1">Name is immutable.</p>
        </div>
        <div>
          <label className="block text-xs text-gray-500 mb-1">Description</label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            disabled={saveMut.isPending}
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
          />
        </div>
        <div>
          <label className="block text-xs text-gray-500 mb-1">Scope</label>
          <div className="flex items-center gap-2">
            {role.tenant_id === null ? (
              <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-purple-50 text-purple-700 border border-purple-200">
                Global
              </span>
            ) : (
              <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-50 text-blue-700 border border-blue-200">
                {scopeLabel}
              </span>
            )}
            <span className="text-xs text-gray-400">Immutable</span>
          </div>
        </div>
        <p className="text-xs text-gray-400">ID: {role.id}</p>
      </div>

      <PermissionSelector
        selected={selected}
        onChange={setSelected}
        disabled={saveMut.isPending}
      />

      <div className="sticky bottom-0 bg-gray-50 -mx-6 px-6 py-3 mt-4 border-t border-gray-200 flex items-center gap-3">
        <Button onClick={() => setConfirmOpen(true)} disabled={!dirty || saveMut.isPending}>
          Save Changes
        </Button>
        <Button variant="outline" onClick={reset} disabled={!dirty || saveMut.isPending}>
          Reset
        </Button>
        {dirty && (
          <span className="text-xs text-gray-500">
            {[...selected].filter((id) => !initial.has(id)).length} to add,{' '}
            {[...initial].filter((id) => !selected.has(id)).length} to remove
            {description !== initialDescription && ', description changed'}
          </span>
        )}
      </div>

      <Dialog.Root open={confirmOpen} onOpenChange={setConfirmOpen}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 bg-black/40" />
          <Dialog.Content className="fixed left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 bg-white rounded-lg shadow-lg p-6 w-[32rem] max-h-[80vh] flex flex-col">
            <Dialog.Title className="text-lg font-semibold mb-1">
              Confirm Role Permissions
            </Dialog.Title>
            <Dialog.Description className="text-sm text-gray-600 mb-4">
              The role <span className="font-medium">{role.name}</span> will have the following{' '}
              {selected.size} permission{selected.size === 1 ? '' : 's'} after saving:
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
              <Button onClick={() => saveMut.mutate()} disabled={saveMut.isPending}>
                {saveMut.isPending ? 'Saving…' : 'Confirm & Save'}
              </Button>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </div>
  )
}
