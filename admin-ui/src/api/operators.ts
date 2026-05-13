import { api } from './client'

export interface Operator {
  id: string
  email: string
  name: string | null
  status: 'active' | 'suspended'
  created_at: string
  updated_at: string
  last_login_at: string | null
}

export const listOperators = () =>
  api.get<{ operators: Operator[]; total: number }>('/operators').then((r) => r.data)

export const createOperator = (data: {
  email: string
  name?: string
  password: string
}) => api.post<Operator>('/operators', data).then((r) => r.data)

export const getOperator = (id: string) =>
  api.get<Operator>(`/operators/${id}`).then((r) => r.data)

export const updateOperator = (
  id: string,
  data: { email?: string; name?: string; status?: 'active' | 'suspended' },
) => api.put<Operator>(`/operators/${id}`, data).then((r) => r.data)

export const deleteOperator = (id: string) => api.delete(`/operators/${id}`)

export const changeOperatorPassword = (id: string, password: string) =>
  api.post(`/operators/${id}/password`, { password })

export const listOperatorRoles = (operatorId: string) =>
  api.get<{ roles: { id: string; name: string; description: string | null }[] }>(
    `/operators/${operatorId}/roles`,
  ).then((r) => r.data)

export const assignRoleToOperator = (operatorId: string, roleId: string) =>
  api.post(`/operators/${operatorId}/roles/${roleId}`, {})

export const revokeRoleFromOperator = (operatorId: string, roleId: string) =>
  api.delete(`/operators/${operatorId}/roles/${roleId}`)
