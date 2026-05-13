import { api } from './client'

export interface Application {
  id: string
  tenant_id: string
  name: string
  client_id: string
  app_type: string
  redirect_uris: string[]
  allowed_scopes: string[]
  grant_types: string[]
  access_token_ttl: number
  refresh_token_ttl: number
  created_at: string
  updated_at: string
}

export const listApplications = (tenantId: string, limit = 50, offset = 0) =>
  api.get<{ applications: Application[]; total: number }>('/applications', {
    params: { tenant_id: tenantId, limit, offset },
  }).then((r) => r.data)

export const getApplication = (tenantId: string, id: string) =>
  api.get<Application>(`/tenants/${tenantId}/applications/${id}`).then((r) => r.data)

export const createApplication = (data: {
  tenant_id: string
  name: string
  client_id: string
  app_type: string
  redirect_uris: string[]
  allowed_scopes: string[]
  grant_types: string[]
}) => api.post<Application>('/applications', data).then((r) => r.data)

export const updateApplication = (tenantId: string, id: string, data: Partial<Application>) =>
  api.put<Application>(`/tenants/${tenantId}/applications/${id}`, data).then((r) => r.data)

export const deleteApplication = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/applications/${id}`)
