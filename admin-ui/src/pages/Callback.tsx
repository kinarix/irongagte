import { useEffect } from 'react'
import { useNavigate } from '@tanstack/react-router'

/// Legacy OAuth callback. Retained because the router still references it but
/// the operator flow no longer uses PKCE — redirect anyone landing here back
/// to /login.
export default function Callback() {
  const navigate = useNavigate()
  useEffect(() => {
    void navigate({ to: '/login' })
  }, [navigate])
  return null
}
