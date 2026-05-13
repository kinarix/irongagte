import { api } from './client'

export interface User {
  id: string
  tenant_id: string
  email: string
  email_verified: boolean
  name: string | null
  given_name: string | null
  family_name: string | null
  picture_url: string | null
  status: string
  attributes: Record<string, any>
  created_at: string
  updated_at: string
  last_login_at: string | null
}

export const listUsers = (tenantId: string, limit = 50, offset = 0) =>
  api.get<{ users: User[]; total: number }>('/users', {
    params: { tenant_id: tenantId, limit, offset },
  }).then((r) => r.data)

export const getUser = (tenantId: string, id: string) =>
  api.get<User>(`/tenants/${tenantId}/users/${id}`).then((r) => r.data)

export const createUser = (data: {
  tenant_id: string
  email: string
  name?: string
  attributes?: Record<string, any>
}) => api.post<User>('/users', data).then((r) => r.data)

export const updateUser = (tenantId: string, id: string, data: Partial<User> & { attributes?: Record<string, any> }) =>
  api.put<User>(`/tenants/${tenantId}/users/${id}`, data).then((r) => r.data)

export const deleteUser = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/users/${id}`)

export const suspendUser = (tenantId: string, id: string) =>
  api.post<User>(`/tenants/${tenantId}/users/${id}/suspend`).then((r) => r.data)

export const unsuspendUser = (tenantId: string, id: string) =>
  api.post<User>(`/tenants/${tenantId}/users/${id}/unsuspend`).then((r) => r.data)
