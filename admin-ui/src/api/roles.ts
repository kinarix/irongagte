import { api } from './client'

export interface Role {
  id: string
  tenant_id: string
  name: string
  description: string | null
  parent_role_id: string | null
  created_at: string
  updated_at: string
}

export interface Permission {
  id: string
  tenant_id: string
  resource: string
  action: string
  description: string | null
  created_at: string
}

export const listRoles = (tenantId: string) =>
  api.get<{ roles: Role[]; total: number }>('/roles', {
    params: { tenant_id: tenantId },
  }).then((r) => r.data)

export const createRole = (data: { tenant_id: string; name: string; description?: string }) =>
  api.post<Role>('/roles', data).then((r) => r.data)

export const getRole = (tenantId: string, id: string) =>
  api.get<Role>(`/tenants/${tenantId}/roles/${id}`).then((r) => r.data)

export const updateRole = (tenantId: string, id: string, data: Partial<Role>) =>
  api.put<Role>(`/tenants/${tenantId}/roles/${id}`, data).then((r) => r.data)

export const deleteRole = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/roles/${id}`)

export const listRolePermissions = (tenantId: string, roleId: string) =>
  api.get<{ permissions: Permission[] }>(`/tenants/${tenantId}/roles/${roleId}/permissions`)
    .then((r) => r.data)

export const assignPermission = (tenantId: string, roleId: string, permissionId: string) =>
  api.post(`/tenants/${tenantId}/roles/${roleId}/permissions`, { permission_id: permissionId })

export const removePermission = (tenantId: string, roleId: string, permId: string) =>
  api.delete(`/tenants/${tenantId}/roles/${roleId}/permissions/${permId}`)

export const listPermissions = (tenantId: string) =>
  api.get<{ permissions: Permission[]; total: number }>('/permissions', {
    params: { tenant_id: tenantId },
  }).then((r) => r.data)

export const createPermission = (data: {
  tenant_id: string
  resource: string
  action: string
  description?: string
}) => api.post<Permission>('/permissions', data).then((r) => r.data)
