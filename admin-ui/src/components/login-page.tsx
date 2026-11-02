import { useState } from 'react'
import { ArrowRight, Eye, EyeOff, KeyRound, Layers3, ShieldCheck, Activity } from 'lucide-react'
import { storage } from '@/lib/storage'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Brand } from '@/components/brand'

interface LoginPageProps {
  onLogin: (apiKey: string) => void
}

export function LoginPage({ onLogin }: LoginPageProps) {
  const [apiKey, setApiKey] = useState('')
  const [showKey, setShowKey] = useState(false)
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (!apiKey.trim()) return
    storage.setApiKey(apiKey.trim())
    onLogin(apiKey.trim())
  }
  return (
    <div className="login-page">
      <header className="login-header"><Brand /></header>
      <main className="login-layout">
        <section className="login-intro" aria-labelledby="intro-title">
          <span className="eyebrow"><span className="status-dot" /> KIRO ACCOUNT CONSOLE</span>
          <h1 id="intro-title">让每一个账号，<br /><span>有序连接。</span></h1>
          <p className="login-description">账号、凭据与额度，集中管理。<br />为你的每一次调用，保持清晰与从容。</p>
          <div className="login-features">
            <div><Layers3 size={19} /><span>多账号管理</span></div>
            <div><Activity size={19} /><span>额度与状态查询</span></div>
            <div><ShieldCheck size={19} /><span>独立密钥访问</span></div>
          </div>
          <div className="login-illustration" aria-hidden="true">
            <div className="illustration-cloud"><img src={`${import.meta.env.BASE_URL}cloud-logo.png`} alt="" /></div>
            <div className="illustration-rule" />
            <div className="illustration-nodes"><span /><span /><span /></div>
          </div>
        </section>
        <section className="login-panel" aria-labelledby="login-title">
          <div className="login-key-icon"><KeyRound size={24} /></div>
          <h2 id="login-title">欢迎回来</h2>
          <p className="login-panel-description">使用管理密钥，进入你的 Kiro 工作空间。</p>
          <form onSubmit={handleSubmit} className="login-form">
            <label htmlFor="admin-key">管理密钥 <span>Admin API Key</span></label>
            <div className="login-input-wrap">
              <Input id="admin-key" type={showKey ? 'text' : 'password'} autoComplete="off" spellCheck={false}
                placeholder="输入你的管理密钥" value={apiKey} onChange={e => setApiKey(e.target.value)} required className="pr-12" />
              <button type="button" className="key-visibility" onClick={() => setShowKey(!showKey)} aria-label={showKey ? '隐藏管理密钥' : '显示管理密钥'}>
                {showKey ? <EyeOff size={17} /> : <Eye size={17} />}
              </button>
            </div>
            <Button type="submit" className="login-submit" disabled={!apiKey.trim()}>进入控制台 <ArrowRight size={17} /></Button>
          </form>
          <div className="login-security"><ShieldCheck size={16} /><p>请使用配置中的 Admin API Key。密钥保存在当前浏览器，请勿在共享设备上保持登录。</p></div>
        </section>
      </main>
      <footer className="login-footer"><span>问高云 · Kiro 控制台</span><span>连接有序，管理从容。</span></footer>
    </div>
  )
}
