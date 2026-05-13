import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { getUser, updateUser } from '@/api/users'
import {
  assignUserClaim,
  listClaimDefinitions,
  listUserClaims,
  previewEffectiveClaims,
  revokeUserClaim,
} from '@/api/claims'
import { listApplications } from '@/api/applications'
import { ArrowLeft } from 'lucide-react'

export default function UserDetail() {
  const { userId } = useParams({ strict: false }) as { userId: string }
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const [attributesJson, setAttributesJson] = useState('')
  const [selectedDef, setSelectedDef] = useState('')
  const [claimValue, setClaimValue] = useState('')
  const [previewAppId, setPreviewAppId] = useState('')

  const { data: user } = useQuery({
    queryKey: ['user', tenantId, userId],
    queryFn: () => getUser(tenantId!, userId),
    enabled: !!tenantId && !!userId,
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

  const { data: claimsData } = useQuery({
    queryKey: ['user-claims', tenantId, userId],
    queryFn: () => listUserClaims(tenantId!, userId),
    enabled: !!tenantId && !!userId,
  })

  const { data: effective } = useQuery({
    queryKey: ['effective-claims', tenantId, userId, previewAppId],
    queryFn: () =>
      previewEffectiveClaims({
        tenant_id: tenantId!,
        user_id: userId,
        application_id: previewAppId,
      }),
    enabled: !!tenantId && !!userId && !!previewAppId,
  })

  useEffect(() => {
    if (user?.attributes) {
      setAttributesJson(JSON.stringify(user.attributes, null, 2))
    }
  }, [user])

  const updateAttrMut = useMutation({
    mutationFn: () => {
      const attrs = attributesJson.trim() ? JSON.parse(attributesJson) : {}
      return updateUser(tenantId!, userId, { attributes: attrs })
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['user', tenantId, userId] }),
  })

  const assignClaimMut = useMutation({
    mutationFn: () =>
      assignUserClaim({
        tenant_id: tenantId!,
        user_id: userId,
        claim_def_id: selectedDef,
        value: claimValue,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['user-claims', tenantId, userId] })
      void qc.invalidateQueries({ queryKey: ['effective-claims', tenantId, userId] })
      setClaimValue('')
    },
  })

  const revokeClaimMut = useMutation({
    mutationFn: (input: { claim_def_id: string; value: string }) =>
      revokeUserClaim({
        tenant_id: tenantId!,
        user_id: userId,
        claim_def_id: input.claim_def_id,
        value: input.value,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['user-claims', tenantId, userId] })
      void qc.invalidateQueries({ queryKey: ['effective-claims', tenantId, userId] })
    },
  })

  const defs = defsData?.claim_definitions ?? []
  const defsById = new Map(defs.map((d) => [d.id, d]))
  const appsById = new Map((appsData?.applications ?? []).map((a) => [a.id, a]))

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>
  if (!user) return <p className="text-sm text-gray-500">Loading…</p>

  return (
    <div className="max-w-3xl">
      <button
        onClick={() => void navigate({ to: '/users' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Users
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-1">{user.email}</h1>
      <p className="text-sm text-gray-500 mb-6">ID: <span className="font-mono text-xs">{user.id}</span></p>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 grid grid-cols-2 gap-3 text-sm">
        <div>
          <span className="text-gray-500">Name</span>
          <p className="font-medium">{user.name ?? '—'}</p>
        </div>
        <div>
          <span className="text-gray-500">Status</span>
          <p className="font-medium capitalize">{user.status}</p>
        </div>
        <div>
          <span className="text-gray-500">Email verified</span>
          <p className="font-medium">{user.email_verified ? 'Yes' : 'No'}</p>
        </div>
        <div>
          <span className="text-gray-500">Created</span>
          <p className="font-medium">{new Date(user.created_at).toLocaleDateString()}</p>
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6">
        <h2 className="font-medium mb-3">Attributes</h2>
        <textarea
          value={attributesJson}
          onChange={(e) => setAttributesJson(e.target.value)}
          className="w-full border border-gray-300 rounded-md px-3 py-2 text-sm font-mono h-32 resize-none focus:outline-none focus:ring-2 focus:ring-gray-400"
          placeholder="{}"
        />
        <Button
          size="sm"
          onClick={() => updateAttrMut.mutate()}
          disabled={updateAttrMut.isPending}
          className="mt-2"
        >
          {updateAttrMut.isPending ? 'Saving…' : 'Save Attributes'}
        </Button>
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6">
        <h2 className="font-medium mb-1">Direct Claim Assignments</h2>
        <p className="text-xs text-gray-500 mb-4">
          Overrides for this user. Scalar values override group-derived values; multi values merge.
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
              placeholder="e.g. owner, enterprise"
            />
          </div>
          <Button
            onClick={() => assignClaimMut.mutate()}
            disabled={!selectedDef || !claimValue || assignClaimMut.isPending}
          >
            Assign
          </Button>
        </div>

        {claimsData?.user_claims.length ? (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 text-gray-500">
                <th className="text-left py-1.5 font-medium">Claim</th>
                <th className="text-left py-1.5 font-medium">Value</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {claimsData.user_claims.map((c) => {
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
          <p className="text-sm text-gray-500">No direct claim assignments.</p>
        )}
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4">
        <h2 className="font-medium mb-1">Effective Claims Preview</h2>
        <p className="text-xs text-gray-500 mb-4">
          The custom claims that would appear in a token minted for this user against the selected
          application (group-derived + direct, with merge/precedence applied).
        </p>
        <select
          value={previewAppId}
          onChange={(e) => setPreviewAppId(e.target.value)}
          className="w-full border border-gray-300 rounded px-2 py-1.5 text-sm mb-3"
        >
          <option value="">Select an application…</option>
          {appsData?.applications.map((a) => (
            <option key={a.id} value={a.id}>
              {a.name} ({a.claim_prefix})
            </option>
          ))}
        </select>
        {previewAppId && (
          <pre className="bg-gray-50 border border-gray-200 rounded p-3 text-xs font-mono overflow-x-auto">
            {effective ? JSON.stringify(effective.claims, null, 2) : 'Loading…'}
          </pre>
        )}
      </div>
    </div>
  )
}
