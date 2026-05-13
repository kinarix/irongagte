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
}) => api.post<User>('/users', data).then((r) => r.data)

export const updateUser = (tenantId: string, id: string, data: Partial<User>) =>
  api.put<User>(`/tenants/${tenantId}/users/${id}`, data).then((r) => r.data)

export const deleteUser = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/users/${id}`)

export const suspendUser = (tenantId: string, id: string) =>
  api.post<User>(`/tenants/${tenantId}/users/${id}/suspend`).then((r) => r.data)

export const unsuspendUser = (tenantId: string, id: string) =>
  api.post<User>(`/tenants/${tenantId}/users/${id}/unsuspend`).then((r) => r.data)

export const getUserRoles = (tenantId: string, id: string) =>
  api.get<{ roles: { id: string; name: string; description: string | null }[] }>(
    `/tenants/${tenantId}/users/${id}/roles`,
  ).then((r) => r.data)

export const assignRole = (tenantId: string, userId: string, roleId: string) =>
  api.post(`/tenants/${tenantId}/users/${userId}/roles`, { role_id: roleId })

export const removeRole = (tenantId: string, userId: string, roleId: string) =>
  api.delete(`/tenants/${tenantId}/users/${userId}/roles/${roleId}`)
