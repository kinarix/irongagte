import { api } from './client'

export interface Group {
  id: string
  tenant_id: string
  display_name: string
  external_id: string | null
  priority: number
  created_at: string
  updated_at: string
}

export interface GroupMember {
  id: string
  email: string
  name: string | null
}

export const listGroups = (tenantId: string, limit = 100, offset = 0) =>
  api.get<{ groups: Group[]; total: number }>('/groups', {
    params: { tenant_id: tenantId, limit, offset },
  }).then((r) => r.data)

export const createGroup = (data: {
  tenant_id: string
  display_name: string
  external_id?: string
  priority?: number
}) => api.post<Group>('/groups', data).then((r) => r.data)

export const getGroup = (tenantId: string, id: string) =>
  api.get<Group>(`/tenants/${tenantId}/groups/${id}`).then((r) => r.data)

export const updateGroup = (tenantId: string, id: string, data: Partial<Group>) =>
  api.put<Group>(`/tenants/${tenantId}/groups/${id}`, data).then((r) => r.data)

export const deleteGroup = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/groups/${id}`)

export const listGroupMembers = (tenantId: string, id: string) =>
  api.get<{ members: GroupMember[] }>(`/tenants/${tenantId}/groups/${id}/members`)
    .then((r) => r.data)

export const addGroupMember = (tenantId: string, id: string, userId: string) =>
  api.post(`/tenants/${tenantId}/groups/${id}/members`, { user_id: userId })

export const removeGroupMember = (tenantId: string, groupId: string, userId: string) =>
  api.delete(`/tenants/${tenantId}/groups/${groupId}/members/${userId}`)
