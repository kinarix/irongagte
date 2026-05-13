import { api } from './client'

export interface OperatorPermission {
  id: string
  resource: string
  action: string
  description: string | null
  created_at: string
}

export const listOperatorPermissions = () =>
  api.get<{ permissions: OperatorPermission[] }>('/operator-permissions')
    .then((r) => r.data)
