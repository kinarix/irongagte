import { api } from './client'

export type ClaimType = 'scalar' | 'multi'

export interface ClaimDefinition {
  id: string
  application_id: string
  key: string
  claim_type: ClaimType
  description: string | null
  created_at: string
  updated_at: string
}

export interface GroupClaim {
  group_id: string
  claim_def_id: string
  value: string
  created_at: string
}

export interface UserClaim {
  user_id: string
  claim_def_id: string
  value: string
  created_at: string
}

// ── Claim definitions ────────────────────────────────────────────────────────

export const listClaimDefinitions = (tenantId: string, applicationId?: string) =>
  api
    .get<{ claim_definitions: ClaimDefinition[]; total: number }>('/claims/definitions', {
      params: { tenant_id: tenantId, application_id: applicationId },
    })
    .then((r) => r.data)

export const createClaimDefinition = (data: {
  tenant_id: string
  application_id: string
  key: string
  claim_type: ClaimType
  description?: string
}) => api.post<ClaimDefinition>('/claims/definitions', data).then((r) => r.data)

export const getClaimDefinition = (tenantId: string, id: string) =>
  api
    .get<ClaimDefinition>(`/tenants/${tenantId}/claims/definitions/${id}`)
    .then((r) => r.data)

export const updateClaimDefinition = (
  tenantId: string,
  id: string,
  data: Partial<Pick<ClaimDefinition, 'key' | 'claim_type' | 'description'>>,
) =>
  api
    .put<ClaimDefinition>(`/tenants/${tenantId}/claims/definitions/${id}`, data)
    .then((r) => r.data)

export const deleteClaimDefinition = (tenantId: string, id: string) =>
  api.delete(`/tenants/${tenantId}/claims/definitions/${id}`)

// ── Group claim assignments ──────────────────────────────────────────────────

export const listGroupClaims = (tenantId: string, groupId: string) =>
  api
    .get<{ group_claims: GroupClaim[]; total: number }>('/claims/group-assignments', {
      params: { tenant_id: tenantId, group_id: groupId },
    })
    .then((r) => r.data)

export const assignGroupClaim = (data: {
  tenant_id: string
  group_id: string
  claim_def_id: string
  value: string
}) => api.post<GroupClaim>('/claims/group-assignments', data).then((r) => r.data)

export const revokeGroupClaim = (data: {
  tenant_id: string
  group_id: string
  claim_def_id: string
  value: string
}) => api.delete('/claims/group-assignments', { data })

// ── User claim assignments ───────────────────────────────────────────────────

export const listUserClaims = (tenantId: string, userId: string) =>
  api
    .get<{ user_claims: UserClaim[]; total: number }>('/claims/user-assignments', {
      params: { tenant_id: tenantId, user_id: userId },
    })
    .then((r) => r.data)

export const assignUserClaim = (data: {
  tenant_id: string
  user_id: string
  claim_def_id: string
  value: string
}) => api.post<UserClaim>('/claims/user-assignments', data).then((r) => r.data)

export const revokeUserClaim = (data: {
  tenant_id: string
  user_id: string
  claim_def_id: string
  value: string
}) => api.delete('/claims/user-assignments', { data })

// ── Effective claims preview ─────────────────────────────────────────────────

export const previewEffectiveClaims = (params: {
  tenant_id: string
  user_id: string
  application_id: string
}) =>
  api
    .get<{ claims: Record<string, string | string[]> }>('/claims/effective', { params })
    .then((r) => r.data)
