import { useQuery } from '@tanstack/react-query'
import { listOperatorPermissions } from '@/api/operatorPermissions'

export default function OperatorPermissionList() {
  const { data, isLoading } = useQuery({
    queryKey: ['operator-permissions'],
    queryFn: listOperatorPermissions,
  })

  const grouped = data?.permissions.reduce(
    (acc, p) => {
      if (!acc[p.resource]) acc[p.resource] = []
      acc[p.resource].push(p)
      return acc
    },
    {} as Record<string, typeof data.permissions>,
  ) ?? {}

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-gray-900">Operator Permissions</h1>
        <p className="text-sm text-gray-500 mt-1">System-wide permissions catalog for operator roles.</p>
      </div>

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <div className="space-y-6">
          {Object.entries(grouped).map(([resource, perms]) => (
            <div key={resource} className="bg-white border border-gray-200 rounded-lg overflow-hidden">
              <div className="bg-gray-50 border-b border-gray-200 px-4 py-2.5">
                <h2 className="font-medium text-sm">{resource}</h2>
              </div>
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-200 bg-gray-50">
                    <th className="text-left px-4 py-2.5 font-medium text-gray-500">Action</th>
                    <th className="text-left px-4 py-2.5 font-medium text-gray-500">Description</th>
                  </tr>
                </thead>
                <tbody>
                  {perms.map((p) => (
                    <tr key={p.id} className="border-b border-gray-100 hover:bg-gray-50">
                      <td className="px-4 py-2.5 font-mono text-sm">{p.action}</td>
                      <td className="px-4 py-2.5 text-gray-600">{p.description ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))}
          {!data?.permissions.length && (
            <p className="text-sm text-gray-500 text-center py-8">No permissions defined.</p>
          )}
        </div>
      )}
    </div>
  )
}
