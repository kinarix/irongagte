import { useState, type FormEvent } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { login } from '@/auth'
import { Button } from '@/components/ui/button'
import { Logo } from '@/components/Logo'

export default function Login() {
  const navigate = useNavigate()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setError(null)
    setPending(true)
    try {
      await login(email, password)
      void navigate({ to: '/users' })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setPending(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <form
        onSubmit={onSubmit}
        className="bg-white rounded-lg border border-gray-200 shadow-sm p-8 w-96 space-y-4"
      >
        <div className="flex flex-col items-center text-center">
          <Logo size={40} />
          <h1 className="text-xl font-semibold text-gray-900 mt-3 mb-1">Irongate Admin</h1>
          <p className="text-sm text-gray-500">Operator sign-in.</p>
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Email</label>
          <input
            type="email"
            value={email}
            autoComplete="username"
            autoFocus
            onChange={(e) => setEmail(e.target.value)}
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
            required
          />
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Password</label>
          <input
            type="password"
            value={password}
            autoComplete="current-password"
            onChange={(e) => setPassword(e.target.value)}
            className="w-full border border-gray-300 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-gray-400"
            required
          />
        </div>

        {error && <p className="text-sm text-red-600">{error}</p>}

        <Button type="submit" className="w-full" disabled={pending}>
          {pending ? 'Signing in…' : 'Sign in'}
        </Button>
      </form>
    </div>
  )
}
