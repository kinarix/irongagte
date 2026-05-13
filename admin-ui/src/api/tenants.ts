import { api } from './client'

export interface Tenant {
  id: string
  name: string
  slug: string
  created_at: string
  updated_at: string
}

export const listTenants = (limit = 50, offset = 0) =>
  api.get<{ tenants: Tenant[]; total: number }>('/tenants', {
    params: { limit, offset },
  }).then((r) => r.data)

export const getTenant = (id: string) =>
  api.get<Tenant>(`/tenants/${id}`).then((r) => r.data)

export const createTenant = (data: { name: string; slug: string }) =>
  api.post<Tenant>('/tenants', data).then((r) => r.data)

export const updateTenant = (id: string, data: Partial<Tenant>) =>
  api.put<Tenant>(`/tenants/${id}`, data).then((r) => r.data)

export const deleteTenant = (id: string) =>
  api.delete(`/tenants/${id}`)
