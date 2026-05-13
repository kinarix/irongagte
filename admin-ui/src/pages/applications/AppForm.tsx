import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { useTenant } from '@/context/tenant'
import { Button } from '@/components/ui/button'
import { getApplication, createApplication, updateApplication } from '@/api/applications'
import { ArrowLeft } from 'lucide-react'

const GRANT_TYPES = ['authorization_code', 'client_credentials', 'refresh_token', 'device_code']
const APP_TYPES = ['web', 'spa', 'native', 'service']

export default function AppForm() {
  const { appId } = useParams({ strict: false }) as { appId?: string }
  const isEdit = !!appId
  const { tenantId } = useTenant()
  const navigate = useNavigate()
  const qc = useQueryClient()

  const [name, setName] = useState('')
  const [clientId, setClientId] = useState('')
  const [appType, setAppType] = useState('web')
  const [redirectUris, setRedirectUris] = useState('')
  const [allowedScopes, setAllowedScopes] = useState('openid profile email')
  const [grantTypes, setGrantTypes] = useState<string[]>(['authorization_code', 'refresh_token'])

  const { data: existing } = useQuery({
    queryKey: ['application', tenantId, appId],
    queryFn: () => getApplication(tenantId!, appId!),
    enabled: isEdit && !!tenantId && !!appId,
  })

  useEffect(() => {
    if (existing) {
      setName(existing.name)
      setClientId(existing.client_id)
      setAppType(existing.app_type)
      setRedirectUris(existing.redirect_uris.join('\n'))
      setAllowedScopes(existing.allowed_scopes.join(' '))
      setGrantTypes(existing.grant_types)
    }
  }, [existing])

  const saveMut = useMutation({
    mutationFn: () => {
      const payload = {
        name,
        client_id: clientId,
        app_type: appType,
        redirect_uris: redirectUris.split('\n').map((s) => s.trim()).filter(Boolean),
        allowed_scopes: allowedScopes.split(/\s+/).filter(Boolean),
        grant_types: grantTypes,
      }
      if (isEdit) {
        return updateApplication(tenantId!, appId!, payload)
      }
      return createApplication({ tenant_id: tenantId!, ...payload })
    },
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['applications', tenantId] })
      void navigate({ to: '/applications' })
    },
  })

  function toggleGrant(g: string) {
    setGrantTypes((prev) => (prev.includes(g) ? prev.filter((x) => x !== g) : [...prev, g]))
  }

  if (!tenantId) return <p className="text-sm text-gray-500">Select a tenant first.</p>

  return (
    <div className="max-w-xl">
      <button
        onClick={() => void navigate({ to: '/applications' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Applications
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-6">
        {isEdit ? 'Edit Application' : 'New Application'}
      </h1>

      <div className="bg-white border border-gray-200 rounded-lg p-5 space-y-4">
        <Field label="Name *">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className={inputCls}
            placeholder="My App"
          />
        </Field>

        <Field label="Client ID *">
          <input
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            className={inputCls}
            placeholder="my-app"
            disabled={isEdit}
          />
        </Field>

        <Field label="App Type">
          <select
            value={appType}
            onChange={(e) => setAppType(e.target.value)}
            className={inputCls}
          >
            {APP_TYPES.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Redirect URIs (one per line)">
          <textarea
            value={redirectUris}
            onChange={(e) => setRedirectUris(e.target.value)}
            className={`${inputCls} h-20 resize-none`}
            placeholder="https://example.com/callback"
          />
        </Field>

        <Field label="Allowed Scopes (space-separated)">
          <input
            value={allowedScopes}
            onChange={(e) => setAllowedScopes(e.target.value)}
            className={inputCls}
          />
        </Field>

        <Field label="Grant Types">
          <div className="space-y-1.5">
            {GRANT_TYPES.map((g) => (
              <label key={g} className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={grantTypes.includes(g)}
                  onChange={() => toggleGrant(g)}
                />
                {g}
              </label>
            ))}
          </div>
        </Field>

        <div className="flex gap-2 pt-2">
          <Button onClick={() => saveMut.mutate()} disabled={!name || !clientId || saveMut.isPending}>
            {saveMut.isPending ? 'Saving…' : 'Save'}
          </Button>
          <Button variant="outline" onClick={() => void navigate({ to: '/applications' })}>
            Cancel
          </Button>
        </div>
        {saveMut.isError && (
          <p className="text-sm text-red-600">
            {(saveMut.error as Error).message}
          </p>
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
