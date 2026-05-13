import axios from 'axios'
import { getAccessToken } from '@/auth'

export const api = axios.create({
  baseURL: '/admin/api/v1',
})

api.interceptors.request.use((config) => {
  const token = getAccessToken()
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})
