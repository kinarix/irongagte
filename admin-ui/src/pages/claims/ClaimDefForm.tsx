import { useEffect, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate, useParams } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import {
  createClaimDefinition,
  getClaimDefinition,
  updateClaimDefinition,
  type ClaimType,
} from '@/api/claims'
import { listApplications } from '@/api/applications'
import axios from 'axios'

function extractErr(e: unknown): string | null {
  if (axios.isAxiosError(e)) {
    const data = e.response?.data as { error?: string } | undefined
    return data?.error ?? e.message
  }
  if (e instanceof Error) return e.message
  return null
}

export default function ClaimDefForm() {
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const params = useParams({ strict: false }) as { claimId?: string }
  const isEdit = !!params.claimId

  const [applicationId, setApplicationId] = useState('')
  const [key, setKey] = useState('')
  const [claimType, setClaimType] = useState<ClaimType>('multi')
  const [description, setDescription] = useState('')
  const [error, setError] = useState<string | null>(null)

  const appsQuery = useQuery({
    queryKey: ['applications', tenantId],
    queryFn: () => listApplications(tenantId!),
    enabled: !!tenantId,
  })

  const existingQuery = useQuery({
    queryKey: ['claim-definition', tenantId, params.claimId],
    queryFn: () => getClaimDefinition(tenantId!, params.claimId!),
    enabled: isEdit && !!tenantId,
  })

  useEffect(() => {
    if (existingQuery.data) {
      setApplicationId(existingQuery.data.application_id)
      setKey(existingQuery.data.key)
      setClaimType(existingQuery.data.claim_type)
      setDescription(existingQuery.data.description ?? '')
    }
  }, [existingQuery.data])

  const createMut = useMutation({
    mutationFn: () =>
      createClaimDefinition({
        tenant_id: tenantId!,
        application_id: applicationId,
        key,
        claim_type: claimType,
        description: description || undefined,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['claim-definitions', tenantId] })
      void navigate({ to: '/claims' })
    },
    onError: (e: unknown) => setError(extractErr(e) ?? 'Failed to create'),
  })

  const updateMut = useMutation({
    mutationFn: () =>
      updateClaimDefinition(tenantId!, params.claimId!, {
        key,
        claim_type: claimType,
        description: description || undefined,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['claim-definitions', tenantId] })
      void navigate({ to: '/claims' })
    },
    onError: (e: unknown) => setError(extractErr(e) ?? 'Failed to update'),
  })

  const submitting = createMut.isPending || updateMut.isPending
  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    if (!isEdit && !applicationId) {
      setError('Pick an application')
      return
    }
    if (!key) {
      setError('Key is required')
      return
    }
    if (isEdit) updateMut.mutate()
    else createMut.mutate()
  }

  if (!tenantId) {
    return <p className="text-sm text-gray-500">Select a tenant first.</p>
  }

  return (
    <div className="max-w-2xl">
      <h1 className="text-xl font-semibold text-gray-900 mb-6">
        {isEdit ? 'Edit Claim Definition' : 'New Claim Definition'}
      </h1>

      <form onSubmit={onSubmit} className="space-y-4 bg-white border border-gray-200 rounded-lg p-6">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Application</label>
          <select
            value={applicationId}
            onChange={(e) => setApplicationId(e.target.value)}
            disabled={isEdit}
            className="w-full border border-gray-300 rounded px-3 py-2 text-sm disabled:bg-gray-50"
          >
            <option value="">Select…</option>
            {appsQuery.data?.applications.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name} ({a.claim_prefix})
              </option>
            ))}
          </select>
          {isEdit && (
            <p className="text-xs text-gray-500 mt-1">
              Cannot move a claim between applications. Delete and recreate instead.
            </p>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Key</label>
          <input
            type="text"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="e.g. roles, plan, region"
            className="w-full border border-gray-300 rounded px-3 py-2 text-sm font-mono"
          />
          <p className="text-xs text-gray-500 mt-1">
            JWT field will be <code>&lt;prefix&gt;:{key || 'key'}</code>. Letters,
            digits, underscore, hyphen only.
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Type</label>
          <div className="flex gap-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="claim_type"
                checked={claimType === 'scalar'}
                onChange={() => setClaimType('scalar')}
              />
              <span>
                <strong>scalar</strong> — single value, user-direct overrides group, ties resolved
                by group priority
              </span>
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="radio"
                name="claim_type"
                checked={claimType === 'multi'}
                onChange={() => setClaimType('multi')}
              />
              <span>
                <strong>multi</strong> — array of values, all group + user values merged and deduped
              </span>
            </label>
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-1">Description</label>
          <input
            type="text"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Optional"
            className="w-full border border-gray-300 rounded px-3 py-2 text-sm"
          />
        </div>

        {error && <p className="text-sm text-red-600">{error}</p>}

        <div className="flex items-center gap-3 pt-2">
          <Button type="submit" disabled={submitting}>
            {submitting ? 'Saving…' : isEdit ? 'Save' : 'Create'}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={() => void navigate({ to: '/claims' })}
          >
            Cancel
          </Button>
        </div>
      </form>
    </div>
  )
}
