import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import {
  getGroup,
  listGroupMembers,
  addGroupMember,
  removeGroupMember,
} from '@/api/groups'
import { listUsers } from '@/api/users'
import {
  assignGroupClaim,
  listClaimDefinitions,
  listGroupClaims,
  revokeGroupClaim,
} from '@/api/claims'
import { listApplications } from '@/api/applications'
import { ArrowLeft } from 'lucide-react'

export default function GroupDetail() {
  const { groupId } = useParams({ strict: false }) as { groupId: string }
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()

  const { data: group } = useQuery({
    queryKey: ['group', tenantId, groupId],
    queryFn: () => getGroup(tenantId!, groupId),
    enabled: !!tenantId && !!groupId,
  })

  const { data: membersData } = useQuery({
    queryKey: ['group-members', tenantId, groupId],
    queryFn: () => listGroupMembers(tenantId!, groupId),
    enabled: !!tenantId && !!groupId,
  })

  const { data: allUsers } = useQuery({
    queryKey: ['users', tenantId],
    queryFn: () => listUsers(tenantId!),
    enabled: !!tenantId,
  })

  const { data: claimsData } = useQuery({
    queryKey: ['group-claims', tenantId, groupId],
    queryFn: () => listGroupClaims(tenantId!, groupId),
    enabled: !!tenantId && !!groupId,
  })

  const { data: defsData } = useQuery({
    queryKey: ['claim-definitions', tenantId],
    queryFn: () => listClaimDefinitions(tenantId!),
    enabled: !!tenantId,
  })

  const { data: appsData } = useQuery({
    queryKey: ['applications', tenantId],
    queryFn: () => listApplications(tenantId!),
    enabled: !!tenantId,
  })

  const addMut = useMutation({
    mutationFn: (userId: string) => addGroupMember(tenantId!, groupId, userId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['group-members', tenantId, groupId] }),
  })

  const removeMut = useMutation({
    mutationFn: (userId: string) => removeGroupMember(tenantId!, groupId, userId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['group-members', tenantId, groupId] }),
  })

  const [selectedDef, setSelectedDef] = useState('')
  const [claimValue, setClaimValue] = useState('')

  const assignClaimMut = useMutation({
    mutationFn: () =>
      assignGroupClaim({
        tenant_id: tenantId!,
        group_id: groupId,
        claim_def_id: selectedDef,
        value: claimValue,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['group-claims', tenantId, groupId] })
      setClaimValue('')
    },
  })

  const revokeClaimMut = useMutation({
    mutationFn: (input: { claim_def_id: string; value: string }) =>
      revokeGroupClaim({
        tenant_id: tenantId!,
        group_id: groupId,
        claim_def_id: input.claim_def_id,
        value: input.value,
      }),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['group-claims', tenantId, groupId] }),
  })

  const memberIds = new Set(membersData?.members.map((m) => m.id))
  const nonMembers = allUsers?.users.filter((u) => !memberIds.has(u.id)) ?? []

  const defs = defsData?.claim_definitions ?? []
  const defsById = new Map(defs.map((d) => [d.id, d]))
  const appsById = new Map((appsData?.applications ?? []).map((a) => [a.id, a]))

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>
  if (!group) return <p className="text-sm text-gray-500">Loading…</p>

  return (
    <div className="max-w-3xl">
      <button
        onClick={() => void navigate({ to: '/groups' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Groups
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-1">{group.display_name}</h1>
      <p className="text-sm text-gray-500 mb-6">
        ID: <span className="font-mono text-xs">{group.id}</span> · Priority: {group.priority}
      </p>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-medium">Members</h2>
          {nonMembers.length > 0 && (
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) addMut.mutate(e.target.value)
                e.target.value = ''
              }}
              className="text-sm border border-gray-300 rounded-md px-2 py-1"
            >
              <option value="">Add user…</option>
              {nonMembers.map((u) => (
                <option key={u.id} value={u.id}>
                  {u.email}
                </option>
              ))}
            </select>
          )}
        </div>
        {membersData?.members.length ? (
          <ul className="space-y-1">
            {membersData.members.map((m) => (
              <li key={m.id} className="flex items-center justify-between py-1.5">
                <span className="text-sm">{m.email}</span>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => removeMut.mutate(m.id)}
                  disabled={removeMut.isPending}
                  className="text-red-600 hover:text-red-700"
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-gray-500">No members yet.</p>
        )}
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4">
        <h2 className="font-medium mb-1">Claim Assignments</h2>
        <p className="text-xs text-gray-500 mb-4">
          Members of this group inherit the values below in any token minted for the matching
          application.
        </p>

        <div className="flex items-end gap-2 mb-4">
          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-500 mb-1">Claim</label>
            <select
              value={selectedDef}
              onChange={(e) => setSelectedDef(e.target.value)}
              className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm"
            >
              <option value="">Select…</option>
              {defs.map((d) => {
                const app = appsById.get(d.application_id)
                return (
                  <option key={d.id} value={d.id}>
                    {app?.claim_prefix ?? '?'}:{d.key} ({d.claim_type})
                  </option>
                )
              })}
            </select>
          </div>
          <div className="flex-1">
            <label className="block text-xs font-medium text-gray-500 mb-1">Value</label>
            <input
              value={claimValue}
              onChange={(e) => setClaimValue(e.target.value)}
              className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm font-mono"
              placeholder="e.g. admin, pro, us-east"
            />
          </div>
          <Button
            onClick={() => assignClaimMut.mutate()}
            disabled={!selectedDef || !claimValue || assignClaimMut.isPending}
          >
            Assign
          </Button>
        </div>

        {claimsData?.group_claims.length ? (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 text-gray-500">
                <th className="text-left py-1.5 font-medium">Claim</th>
                <th className="text-left py-1.5 font-medium">Value</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {claimsData.group_claims.map((c) => {
                const def = defsById.get(c.claim_def_id)
                const app = def ? appsById.get(def.application_id) : null
                return (
                  <tr key={`${c.claim_def_id}-${c.value}`} className="border-b border-gray-100">
                    <td className="py-1.5 font-mono text-xs">
                      {app?.claim_prefix ?? '?'}:{def?.key ?? c.claim_def_id}
                    </td>
                    <td className="py-1.5 font-mono text-xs">{c.value}</td>
                    <td className="py-1.5 text-right">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() =>
                          revokeClaimMut.mutate({
                            claim_def_id: c.claim_def_id,
                            value: c.value,
                          })
                        }
                        disabled={revokeClaimMut.isPending}
                        className="text-red-600 hover:text-red-700"
                      >
                        Revoke
                      </Button>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        ) : (
          <p className="text-sm text-gray-500">No claim assignments yet.</p>
        )}
      </div>
    </div>
  )
}
