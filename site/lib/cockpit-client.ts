import { createClient, type NormalizeOAS } from 'fets'
import type openapi from './openapi'

export const client = createClient<NormalizeOAS<typeof openapi>>({})
