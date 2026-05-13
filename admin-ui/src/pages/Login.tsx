import { startPkceFlow } from '@/auth'
import { Button } from '@/components/ui/button'

export default function Login() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="bg-white rounded-lg border border-gray-200 shadow-sm p-8 w-80">
        <h1 className="text-xl font-semibold text-gray-900 mb-1">Irongate Admin</h1>
        <p className="text-sm text-gray-500 mb-6">Sign in to manage your identity system.</p>
        <Button className="w-full" onClick={() => void startPkceFlow()}>
          Sign in with Irongate
        </Button>
      </div>
    </div>
  )
}
