const API_BASE = (import.meta.env.VITE_API_BASE as string | undefined) ?? ''
const LOGIN_ENDPOINT = `${API_BASE}/operator/login`
const LOGOUT_ENDPOINT = `${API_BASE}/operator/logout`

// In-memory token store (never localStorage). Operator tokens are short-lived
// JWTs; re-login on expiry.
let accessToken: string | null = null

export function getAccessToken() {
  return accessToken
}

export function setAccessToken(token: string) {
  accessToken = token
}

export function clearAccessToken() {
  accessToken = null
}

export interface OperatorInfo {
  id: string
  email: string
  name: string | null
}

let currentOperator: OperatorInfo | null = null

export function getCurrentOperator() {
  return currentOperator
}

export async function login(email: string, password: string): Promise<OperatorInfo> {
  const res = await fetch(LOGIN_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || `Login failed (${res.status})`)
  }
  const data = (await res.json()) as {
    access_token: string
    operator: OperatorInfo
  }
  setAccessToken(data.access_token)
  currentOperator = data.operator
  return data.operator
}

export async function logout() {
  try {
    await fetch(LOGOUT_ENDPOINT, {
      method: 'POST',
      headers: accessToken ? { Authorization: `Bearer ${accessToken}` } : {},
    })
  } catch {
    // ignore
  }
  clearAccessToken()
  currentOperator = null
}
