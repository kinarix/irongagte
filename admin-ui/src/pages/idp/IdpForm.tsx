import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { getIdpConfig, createIdpConfig, updateIdpConfig } from '@/api/idp'
import { ArrowLeft } from 'lucide-react'

const PROVIDER_TYPES = ['oidc', 'oauth2', 'ldap']

export default function IdpForm() {
  const { idpId } = useParams({ strict: false }) as { idpId?: string }
  const isEdit = !!idpId
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()

  const [name, setName] = useState('')
  const [providerType, setProviderType] = useState('oidc')
  const [clientId, setClientId] = useState('')
  const [clientSecret, setClientSecret] = useState('')
  const [discoveryUrl, setDiscoveryUrl] = useState('')
  const [enabled, setEnabled] = useState(true)

  const { data: existing } = useQuery({
    queryKey: ['idp-config', tenantId, idpId],
    queryFn: () => getIdpConfig(tenantId!, idpId!),
    enabled: isEdit && !!tenantId && !!idpId,
  })

  useEffect(() => {
    if (existing) {
      setName(existing.name)
      setProviderType(existing.provider_type)
      setClientId(existing.client_id ?? '')
      setDiscoveryUrl(existing.discovery_url ?? '')
      setEnabled(existing.enabled)
    }
  }, [existing])

  const saveMut = useMutation({
    mutationFn: () => {
      const payload = {
        name,
        provider_type: providerType,
        client_id: clientId || undefined,
        client_secret: clientSecret || undefined,
        discovery_url: discoveryUrl || undefined,
      }
      if (isEdit) {
        return updateIdpConfig(tenantId!, idpId!, { ...payload, enabled })
      }
      return createIdpConfig({ tenant_id: tenantId!, ...payload })
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['idp-configs', tenantId] })
      void navigate({ to: '/idp' })
    },
  })

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>

  return (
    <div className="max-w-xl">
      <button
        onClick={() => void navigate({ to: '/idp' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Identity Providers
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-6">
        {isEdit ? 'Edit Identity Provider' : 'New Identity Provider'}
      </h1>

      <div className="bg-white border border-gray-200 rounded-lg p-5 space-y-4">
        <Field label="Name *">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className={inputCls}
            placeholder="e.g. Google, Okta"
          />
        </Field>

        <Field label="Provider Type">
          <select
            value={providerType}
            onChange={(e) => setProviderType(e.target.value)}
            className={inputCls}
            disabled={isEdit}
          >
            {PROVIDER_TYPES.map((t) => (
              <option key={t} value={t}>
                {t.toUpperCase()}
              </option>
            ))}
          </select>
        </Field>

        {(providerType === 'oidc' || providerType === 'oauth2') && (
          <>
            <Field label="Client ID">
              <input
                value={clientId}
                onChange={(e) => setClientId(e.target.value)}
                className={inputCls}
              />
            </Field>
            <Field label={isEdit ? 'Client Secret (leave blank to keep existing)' : 'Client Secret'}>
              <input
                type="password"
                value={clientSecret}
                onChange={(e) => setClientSecret(e.target.value)}
                className={inputCls}
                autoComplete="new-password"
              />
            </Field>
          </>
        )}

        {providerType === 'oidc' && (
          <Field label="Discovery URL">
            <input
              value={discoveryUrl}
              onChange={(e) => setDiscoveryUrl(e.target.value)}
              className={inputCls}
              placeholder="https://accounts.google.com"
            />
          </Field>
        )}

        {isEdit && (
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
            />
            Enabled
          </label>
        )}

        <div className="flex gap-2 pt-2">
          <Button onClick={() => saveMut.mutate()} disabled={!name || saveMut.isPending}>
            {saveMut.isPending ? 'Saving…' : 'Save'}
          </Button>
          <Button variant="outline" onClick={() => void navigate({ to: '/idp' })}>
            Cancel
          </Button>
        </div>
        {saveMut.isError && (
          <p className="text-sm text-red-600">{(saveMut.error as Error).message}</p>
        )}
      </div>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-xs font-medium text-gray-500 mb-1">{label}</label>
      {children}
    </div>
  )
}

const inputCls =
  'w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400 bg-white'
