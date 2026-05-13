import { createContext, useContext, useState, type ReactNode } from 'react'

interface TenantContextValue {
  tenantId: string | null
  setTenantId: (id: string) => void
}

const TenantContext = createContext<TenantContextValue>({
  tenantId: null,
  setTenantId: () => {},
})

export function TenantProvider({ children }: { children: ReactNode }) {
  const [tenantId, setTenantIdState] = useState<string | null>(
    localStorage.getItem('admin_tenant_id'),
  )

  function setTenantId(id: string) {
    localStorage.setItem('admin_tenant_id', id)
    setTenantIdState(id)
  }

  return (
    <TenantContext.Provider value={{ tenantId, setTenantId }}>
      {children}
    </TenantContext.Provider>
  )
}

export const useTenant = () => useContext(TenantContext)
