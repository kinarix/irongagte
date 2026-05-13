import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { getOperator, listOperatorRoles, assignRoleToOperator, revokeRoleFromOperator } from '@/api/operators'
import { listOperatorRoles as listAllOperatorRoles } from '@/api/operatorRoles'
import { ArrowLeft } from 'lucide-react'

export default function OperatorDetail() {
  const { operatorId } = useParams({ strict: false }) as { operatorId: string }
  const navigate = useNavigate()
  const qc = useQueryClient()

  const { data: operator } = useQuery({
    queryKey: ['operator', operatorId],
    queryFn: () => getOperator(operatorId),
    enabled: !!operatorId,
  })

  const { data: opRolesData } = useQuery({
    queryKey: ['operator-roles', operatorId],
    queryFn: () => listOperatorRoles(operatorId),
    enabled: !!operatorId,
  })

  const { data: allRolesData } = useQuery({
    queryKey: ['operator-roles'],
    queryFn: () => listAllOperatorRoles(),
  })

  const assignMut = useMutation({
    mutationFn: (roleId: string) => assignRoleToOperator(operatorId, roleId),
    onSuccess: () =>
      void qc.invalidateQueries({ queryKey: ['operator-roles', operatorId] }),
  })

  const revokeMut = useMutation({
    mutationFn: (roleId: string) => revokeRoleFromOperator(operatorId, roleId),
    onSuccess: () =>
      void qc.invalidateQueries({ queryKey: ['operator-roles', operatorId] }),
  })

  const assignedIds = new Set(opRolesData?.roles.map((r) => r.id))
  const unassigned = allRolesData?.roles.filter((r) => !assignedIds.has(r.id)) ?? []

  if (!operator) return <p className="text-sm text-gray-500">Loading…</p>

  return (
    <div className="max-w-2xl">
      <button
        onClick={() => void navigate({ to: '/operators' })}
        className="flex items-center gap-1 text-sm text-gray-500 hover:text-gray-700 mb-4"
      >
        <ArrowLeft size={14} /> Back to Operators
      </button>

      <h1 className="text-xl font-semibold text-gray-900 mb-1">{operator.email}</h1>
      <p className="text-xs text-gray-400 mb-6">ID: {operator.id}</p>

      <div className="bg-white border border-gray-200 rounded-lg p-4 mb-6 grid grid-cols-2 gap-3 text-sm">
        <div>
          <span className="text-gray-500">Name</span>
          <p className="font-medium">{operator.name ?? '—'}</p>
        </div>
        <div>
          <span className="text-gray-500">Status</span>
          <p className="font-medium capitalize">{operator.status}</p>
        </div>
      </div>

      <div className="bg-white border border-gray-200 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-medium">Operator Roles</h2>
          {unassigned.length > 0 && (
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) assignMut.mutate(e.target.value)
                e.target.value = ''
              }}
              className="text-sm border border-gray-300 rounded-md px-2 py-1"
            >
              <option value="">Assign role…</option>
              {unassigned.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
          )}
        </div>
        {opRolesData?.roles.length ? (
          <ul className="space-y-1">
            {opRolesData.roles.map((r) => (
              <li key={r.id} className="flex items-center justify-between py-1.5">
                <span className="text-sm">{r.name}</span>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => revokeMut.mutate(r.id)}
                  disabled={revokeMut.isPending}
                  className="text-red-600 hover:text-red-700"
                >
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-gray-500">No roles assigned.</p>
        )}
      </div>
    </div>
  )
}
