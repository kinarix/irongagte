import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { listOperatorPermissions } from '@/api/operatorPermissions'
import type { OperatorPermission } from '@/api/operatorRoles'
import { Check, Minus } from 'lucide-react'

type GroupState = 'none' | 'some' | 'all'

function TriCheckbox({
  state,
  onClick,
  disabled,
}: {
  state: GroupState
  onClick: () => void
  disabled?: boolean
}) {
  const base = 'w-4 h-4 rounded border flex items-center justify-center transition-colors'
  const style =
    state === 'all'
      ? 'bg-gray-900 border-gray-900 text-white'
      : state === 'some'
        ? 'bg-yellow-300 border-yellow-400 text-yellow-900'
        : 'bg-white border-gray-300 hover:border-gray-400'
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`${base} ${style} ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
      aria-checked={state === 'all'}
      role="checkbox"
    >
      {state === 'all' && <Check size={12} strokeWidth={3} />}
      {state === 'some' && <Minus size={12} strokeWidth={3} />}
    </button>
  )
}

interface Props {
  selected: Set<string>
  onChange: (next: Set<string>) => void
  disabled?: boolean
  height?: string
}

export function PermissionSelector({
  selected,
  onChange,
  disabled = false,
  height = 'h-[60vh]',
}: Props) {
  const { data } = useQuery({
    queryKey: ['operator-permissions'],
    queryFn: listOperatorPermissions,
  })

  const grouped = useMemo(() => {
    const byResource = new Map<string, OperatorPermission[]>()
    for (const p of data?.permissions ?? []) {
      if (!byResource.has(p.resource)) byResource.set(p.resource, [])
      byResource.get(p.resource)!.push(p)
    }
    return Array.from(byResource.entries()).sort(([a], [b]) => a.localeCompare(b))
  }, [data])

  const groupState = (perms: OperatorPermission[]): GroupState => {
    const total = perms.length
    let count = 0
    for (const p of perms) if (selected.has(p.id)) count++
    if (count === 0) return 'none'
    if (count === total) return 'all'
    return 'some'
  }

  const toggleGroup = (perms: OperatorPermission[]) => {
    const next = new Set(selected)
    const allSelected = perms.every((p) => next.has(p.id))
    if (allSelected) for (const p of perms) next.delete(p.id)
    else for (const p of perms) next.add(p.id)
    onChange(next)
  }

  const toggle = (id: string) => {
    const next = new Set(selected)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    onChange(next)
  }

  return (
    <div className={`bg-white border border-gray-200 rounded-lg p-4 flex flex-col ${height}`}>
      <div className="flex items-center justify-between mb-3 shrink-0">
        <h2 className="font-medium">Permissions</h2>
        <span className="text-xs text-gray-500">
          {selected.size} of {data?.permissions.length ?? 0} selected
        </span>
      </div>
      {grouped.length === 0 ? (
        <p className="text-sm text-gray-500">No permissions in catalog.</p>
      ) : (
        <div className="space-y-5 overflow-y-auto pr-1 -mr-1">
          {grouped.map(([resource, perms]) => {
            const state = groupState(perms)
            return (
              <div key={resource}>
                <div className="flex items-center gap-2 mb-2">
                  <TriCheckbox
                    state={state}
                    onClick={() => toggleGroup(perms)}
                    disabled={disabled}
                  />
                  <span className="text-xs font-semibold uppercase tracking-wider text-gray-500">
                    {resource}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-x-4 gap-y-1.5 sm:grid-cols-3 pl-6">
                  {perms
                    .slice()
                    .sort((a, b) => a.action.localeCompare(b.action))
                    .map((p) => (
                      <label
                        key={p.id}
                        className="flex items-center gap-2 text-sm cursor-pointer hover:text-gray-900 text-gray-700"
                        title={p.description ?? undefined}
                      >
                        <input
                          type="checkbox"
                          checked={selected.has(p.id)}
                          onChange={() => toggle(p.id)}
                          disabled={disabled}
                          className="rounded border-gray-300"
                        />
                        <span className="font-mono">{p.action}</span>
                      </label>
                    ))}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

export function groupSelectedForConfirm(
  selected: Set<string>,
  catalog: OperatorPermission[],
): [string, OperatorPermission[]][] {
  const byResource = new Map<string, OperatorPermission[]>()
  for (const p of catalog) {
    if (!selected.has(p.id)) continue
    if (!byResource.has(p.resource)) byResource.set(p.resource, [])
    byResource.get(p.resource)!.push(p)
  }
  return Array.from(byResource.entries()).sort(([a], [b]) => a.localeCompare(b))
}
