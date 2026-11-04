import { api } from './credentials'

export interface ClientKey {
  id: string
  name: string
  maskedKey: string
  disabled: boolean
  createdAt: string
}
export interface ClientKeyList { keys: ClientKey[]; legacyKeyEnabled: boolean }
export interface CreatedClientKey { key: string; item: ClientKey }
export async function listClientKeys(): Promise<ClientKeyList> {
  return (await api.get<ClientKeyList>('/client-keys')).data
}
export async function createClientKey(name: string): Promise<CreatedClientKey> {
  return (await api.post<CreatedClientKey>('/client-keys', { name })).data
}
export async function setClientKeyDisabled(id: string, disabled: boolean): Promise<ClientKey> {
  return (await api.post<ClientKey>('/client-keys/' + encodeURIComponent(id) + '/disabled', { disabled })).data
}
export async function deleteClientKey(id: string): Promise<void> {
  await api.delete('/client-keys/' + encodeURIComponent(id))
}
