import { useEffect, useRef } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { exchangeCode, setAccessToken } from '@/auth'

export default function Callback() {
  const navigate = useNavigate()
  const ran = useRef(false)

  useEffect(() => {
    if (ran.current) return
    ran.current = true

    const params = new URLSearchParams(window.location.search)
    const code = params.get('code')
    const state = params.get('state')

    if (!code || !state) {
      void navigate({ to: '/login' })
      return
    }

    exchangeCode(code, state)
      .then((token) => {
        setAccessToken(token)
        void navigate({ to: '/users' })
      })
      .catch(() => {
        void navigate({ to: '/login' })
      })
  }, [navigate])

  return (
    <div className="min-h-screen flex items-center justify-center text-sm text-gray-500">
      Completing sign in…
    </div>
  )
}
