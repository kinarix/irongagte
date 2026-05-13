import { api } from './client'

export interface IdpConfig {
  id: string
  tenant_id: string
  provider_type: string
  name: string
  client_id: string | null
  discovery_url: string | null
  enabled: boolean
  created_at: string
  updated_at: string
}

export const listIdpConfigs = (tenantId: string) =>
  api.get<{ configs: IdpConfig[]; total: number }>('/idp-configs', {
    params: { tenant_id: tenantId },
  }).then((r) => r.data)

export const getIdpConfig = (tenantId: string, id: string) =>
  api.get<IdpConfig>(`/tenants/${tenantId}/idp-configs/${id}`).then((r) => r.data)

export const createIdpConfig = (data: {
  tenant_id: string
  provider_type: string
  name: string
  client_id?: string
  client_secret?: string
  discovery_url?: string
}) => api.post<IdpConfig>('/idp-configs', data).then((r) => r.data)

export const updateIdpConfig = (tenantId: string, id: string, data: Partial<IdpConfig>) =>
  api.put<IdpConfig>(`/tenants/${tenantId}/idp-configs/${id}`, data).then((r) => r.data)

export const deleteIdpConfig = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/idp-configs/${id}`)
