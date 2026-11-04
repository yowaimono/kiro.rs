import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ChevronRight, Copy, KeyRound, LogOut, Plus, RefreshCw, ShieldCheck, Trash2 } from 'lucide-react'
import { toast } from 'sonner'
import { Brand } from '@/components/brand'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { createClientKey, deleteClientKey, listClientKeys, setClientKeyDisabled, type ClientKey, type CreatedClientKey } from '@/api/client-keys'
import { extractErrorMessage } from '@/lib/utils'

interface Props { onBack: () => void; onLogout: () => void }

export function ClientKeysPage({ onBack, onLogout }: Props) {
  const queryClient = useQueryClient()
  const query = useQuery({ queryKey: ['client-keys'], queryFn: listClientKeys, retry: false })
  const [createOpen, setCreateOpen] = useState(false)
  const [name, setName] = useState('')
  const [created, setCreated] = useState<CreatedClientKey | null>(null)
  const [creating, setCreating] = useState(false)
  const [createError, setCreateError] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<ClientKey | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [deleteError, setDeleteError] = useState('')
  const keys = query.data?.keys ?? []
  const active = keys.filter(key => !key.disabled).length
  const refresh = () => queryClient.invalidateQueries({ queryKey: ['client-keys'] })
  const closeCreate = () => { setCreateOpen(false); setCreated(null); setName(''); setCreateError('') }
  const changeCreateOpen = (open: boolean) => {
    if (creating) return
    if (!open && created && !window.confirm('完整密钥仅显示这一次。确认已经保存并关闭？')) return
    if (!open) closeCreate(); else setCreateOpen(true)
  }
  const copy = async (value: string, label: string) => {
    try { await navigator.clipboard.writeText(value); toast.success(label + '已复制') }
    catch { toast.error('复制失败，请选中文字手动复制') }
  }
  const submit = async (event: React.FormEvent) => {
    event.preventDefault()
    if (creating || !name.trim()) return
    setCreating(true); setCreateError('')
    try { const result = await createClientKey(name.trim()); setCreated(result); void refresh() }
    catch (error) { setCreateError(extractErrorMessage(error)) }
    finally { setCreating(false) }
  }
  const toggle = async (key: ClientKey) => {
    setBusyId(key.id)
    try { await setClientKeyDisabled(key.id, !key.disabled); await refresh(); toast.success(key.disabled ? '密钥已启用' : '密钥已禁用') }
    catch (error) { toast.error(extractErrorMessage(error)) }
    finally { setBusyId(null) }
  }
  const remove = async () => {
    if (!deleteTarget || busyId) return
    setBusyId(deleteTarget.id); setDeleteError('')
    try { await deleteClientKey(deleteTarget.id); await refresh(); setDeleteTarget(null); toast.success('密钥已删除') }
    catch (error) { setDeleteError(extractErrorMessage(error)) }
    finally { setBusyId(null) }
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="console-header"><div className="console-header-inner">
        <Brand />
        <div className="header-actions">
          <Button className="client-key-nav" variant="ghost" size="sm" onClick={onBack}><ArrowLeft size={15} /> 账号管理</Button>
          <span className="header-action-divider" />
          <Button variant="ghost" size="icon" onClick={onLogout} aria-label="退出登录" title="退出登录"><LogOut /></Button>
        </div>
      </div></header>
      <main className="console-main client-keys-page">
        <div className="console-breadcrumb"><span>工作空间</span><ChevronRight size={13} /><span>客户端密钥</span></div>
        <div className="page-heading">
          <div><h1>客户端密钥</h1><p>为 Claude Code、应用和团队成员创建独立的调用凭证。</p></div>
          <Button onClick={() => { closeCreate(); setCreateOpen(true) }}><Plus size={16} /> 创建密钥</Button>
        </div>
        <div className="client-key-connection">
          <div><span className="client-key-eyebrow">ANTHROPIC 兼容接口</span><code>{window.location.origin}</code></div>
          <Button variant="outline" size="sm" onClick={() => copy(window.location.origin, '接口地址')}><Copy size={14} /> 复制地址</Button>
        </div>
        <div className="client-key-notice"><ShieldCheck size={18} /><p>客户端 Key 仅用于调用 API，不能登录管理后台。所有 Key 共用当前账号池，暂不提供独立额度和模型分组。</p></div>
        {query.data?.legacyKeyEnabled && <p className="client-key-legacy">原配置文件中的全局调用 Key 仍然有效，不在此列表中；需要停用或更换时请修改服务配置。</p>}
        <section className="credentials-section" aria-labelledby="client-key-list-title">
          <div className="credentials-toolbar">
            <div><h2 className="section-title" id="client-key-list-title">调用密钥 <span>{keys.length}</span></h2><p className="section-description">{active} 把已启用 · 完整密钥仅在创建成功时显示一次</p></div>
            <Button variant="outline" size="sm" onClick={() => void query.refetch()} disabled={query.isFetching} aria-label="刷新密钥列表"><RefreshCw size={14} className={query.isFetching ? 'animate-spin' : ''} /> 刷新</Button>
          </div>
          {query.isLoading ? <div className="client-key-state" role="status"><RefreshCw className="animate-spin" size={22} /><p>正在加载客户端密钥…</p></div>
            : query.isError ? <div className="client-key-state" role="alert"><h3>无法加载密钥</h3><p>{extractErrorMessage(query.error)}</p><div className="empty-actions"><Button variant="outline" onClick={() => void query.refetch()}>重试</Button><Button variant="ghost" onClick={onLogout}>重新登录</Button></div></div>
            : keys.length === 0 ? <div className="empty-state client-key-empty"><div className="empty-icon"><KeyRound size={30} strokeWidth={1.4} /></div><h3>创建第一把调用密钥</h3><p>按应用或成员命名，便于独立停用和更换。</p><Button onClick={() => setCreateOpen(true)}><Plus size={16} /> 创建密钥</Button></div>
            : <div className="client-key-list">
              <div className="client-key-row client-key-table-heading" aria-hidden="true"><span>名称 / 调用 Key</span><span>状态</span><span>创建时间</span><span>操作</span></div>
              {keys.map(key => <div className="client-key-row" key={key.id} data-testid="client-key-row">
                <div className="client-key-identity"><strong title={key.name}>{key.name}</strong><code>{key.maskedKey}</code></div>
                <Badge variant={key.disabled ? 'secondary' : 'default'}>{key.disabled ? '已禁用' : '已启用'}</Badge>
                <time dateTime={key.createdAt}>{new Date(key.createdAt).toLocaleString('zh-CN', { hour12: false })}</time>
                <div className="client-key-row-actions"><Button size="sm" variant="outline" disabled={busyId !== null} onClick={() => void toggle(key)} aria-label={(key.disabled ? '启用 ' : '禁用 ') + key.name}>{busyId === key.id ? '处理中…' : key.disabled ? '启用' : '禁用'}</Button><Button size="sm" variant="ghost" disabled={busyId !== null} onClick={() => { setDeleteError(''); setDeleteTarget(key) }} aria-label={'删除 ' + key.name}><Trash2 size={14} /> 删除</Button></div>
              </div>)}
            </div>}
        </section>
        <footer className="console-footer"><span>问高云 · Kiro 控制台</span><span>仅保存密钥摘要 · 变更立即生效</span></footer>
      </main>
      <Dialog open={createOpen} onOpenChange={changeCreateOpen}>
        <DialogContent>
          <DialogHeader><DialogTitle>{created ? '密钥已创建' : '创建客户端密钥'}</DialogTitle><DialogDescription>{created ? '请立即复制并妥善保存，关闭后无法再次查看完整密钥。' : '使用应用或成员名称，便于后续识别和停用。'}</DialogDescription></DialogHeader>
          {created ? <div className="client-key-created">
            <label htmlFor="created-client-key">{created.item.name}</label>
            <textarea id="created-client-key" value={created.key} readOnly rows={3} spellCheck={false} aria-label="新客户端密钥" onFocus={event => event.currentTarget.select()} />
            <Button variant="outline" onClick={() => copy(created.key, '密钥')}><Copy size={15} /> 复制完整密钥</Button>
            <p>请求头使用 <code>x-api-key</code> 或 <code>Authorization: Bearer</code>。</p>
            <DialogFooter><Button onClick={closeCreate}>我已保存</Button></DialogFooter>
          </div> : <form onSubmit={submit} className="client-key-create-form">
            <label htmlFor="client-key-name">密钥名称</label><Input id="client-key-name" autoFocus value={name} onChange={event => setName(event.target.value)} maxLength={80} required placeholder="例如：Claude Code · 我的电脑" disabled={creating} aria-describedby={createError ? 'client-key-create-error' : undefined} />
            {createError && <p id="client-key-create-error" className="text-destructive text-sm" role="alert">{createError}</p>}
            <DialogFooter><Button type="button" variant="outline" onClick={closeCreate} disabled={creating}>取消</Button><Button type="submit" disabled={creating || !name.trim()}>{creating ? '创建中…' : '创建密钥'}</Button></DialogFooter>
          </form>}
        </DialogContent>
      </Dialog>
      <Dialog open={deleteTarget !== null} onOpenChange={open => { if (!open && !busyId) setDeleteTarget(null) }}>
        <DialogContent><DialogHeader><DialogTitle>删除客户端密钥</DialogTitle><DialogDescription>删除“{deleteTarget?.name}”后，该密钥将立即失效且无法恢复。只想暂停调用时，可以先禁用。</DialogDescription></DialogHeader>
          {deleteError && <p role="alert" className="text-destructive text-sm">{deleteError}</p>}
          <DialogFooter><Button variant="outline" disabled={busyId !== null} onClick={() => setDeleteTarget(null)}>取消</Button><Button variant="destructive" disabled={busyId !== null} onClick={() => void remove()}>{busyId ? '删除中…' : '确认删除'}</Button></DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
