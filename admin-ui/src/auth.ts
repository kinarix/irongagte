const CLIENT_ID = 'irongate-admin'
const REDIRECT_URI = `${window.location.origin}/admin/callback`
const SCOPES = 'openid admin:*'
const API_BASE = (import.meta.env.VITE_API_BASE as string | undefined) ?? ''
const AUTH_ENDPOINT = `${API_BASE}/oauth2/authorize`
const TOKEN_ENDPOINT = `${API_BASE}/oauth2/token`

// In-memory token store (never localStorage)
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

function randomBase64url(byteLength: number): string {
  const arr = crypto.getRandomValues(new Uint8Array(byteLength))
  return btoa(String.fromCharCode(...arr))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '')
}

async function sha256Base64url(plain: string): Promise<string> {
  const enc = new TextEncoder().encode(plain)
  const hash = await crypto.subtle.digest('SHA-256', enc)
  return btoa(String.fromCharCode(...new Uint8Array(hash)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '')
}

export async function startPkceFlow() {
  const verifier = randomBase64url(48)
  const challenge = await sha256Base64url(verifier)
  const state = randomBase64url(16)

  sessionStorage.setItem('pkce_verifier', verifier)
  sessionStorage.setItem('pkce_state', state)

  const params = new URLSearchParams({
    response_type: 'code',
    client_id: CLIENT_ID,
    redirect_uri: REDIRECT_URI,
    scope: SCOPES,
    state,
    code_challenge: challenge,
    code_challenge_method: 'S256',
  })

  window.location.href = `${AUTH_ENDPOINT}?${params}`
}

export async function exchangeCode(code: string, returnedState: string): Promise<string> {
  const verifier = sessionStorage.getItem('pkce_verifier')
  const savedState = sessionStorage.getItem('pkce_state')

  if (!verifier || savedState !== returnedState) {
    throw new Error('PKCE state mismatch')
  }

  sessionStorage.removeItem('pkce_verifier')
  sessionStorage.removeItem('pkce_state')

  const body = new URLSearchParams({
    grant_type: 'authorization_code',
    code,
    redirect_uri: REDIRECT_URI,
    client_id: CLIENT_ID,
    code_verifier: verifier,
  })

  const res = await fetch(TOKEN_ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  })

  if (!res.ok) {
    const text = await res.text()
    throw new Error(`Token exchange failed: ${text}`)
  }

  const data = await res.json()
  return data.access_token as string
}
