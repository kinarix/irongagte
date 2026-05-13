import { api } from './client'

export interface OperatorPermission {
  id: string
  resource: string
  action: string
  description: string | null
  created_at: string
}

export interface OperatorRole {
  id: string
  /** null = global (cross-tenant) role; otherwise scoped to the named tenant */
  tenant_id: string | null
  name: string
  description: string | null
  created_at: string
  updated_at: string
}

export interface ListOperatorRolesParams {
  /** Server-side filter: 'global', a tenant UUID, or omitted for all visible. */
  scope?: 'global'
  tenant_id?: string
}

export const listOperatorRoles = (params: ListOperatorRolesParams = {}) =>
  api
    .get<{ roles: OperatorRole[]; total: number }>('/operator-roles', { params })
    .then((r) => r.data)

export const getOperatorRole = (id: string) =>
  api.get<OperatorRole>(`/operator-roles/${id}`).then((r) => r.data)

export const createOperatorRole = (data: {
  name: string
  description?: string
  /** Omit for a global role. */
  tenant_id?: string
}) => api.post<OperatorRole>('/operator-roles', data).then((r) => r.data)

export const updateOperatorRole = (id: string, data: Partial<OperatorRole>) =>
  api.put<OperatorRole>(`/operator-roles/${id}`, data).then((r) => r.data)

export const deleteOperatorRole = (id: string) =>
  api.delete(`/operator-roles/${id}`)

export const listRolePermissions = (roleId: string) =>
  api.get<{ permissions: OperatorPermission[] }>(`/operator-roles/${roleId}/permissions`)
    .then((r) => r.data)

export const assignPermissionToRole = (roleId: string, permissionId: string) =>
  api.post(`/operator-roles/${roleId}/permissions/${permissionId}`, {})

export const revokePermissionFromRole = (roleId: string, permissionId: string) =>
  api.delete(`/operator-roles/${roleId}/permissions/${permissionId}`)
