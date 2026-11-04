import { useState, useEffect } from 'react'
import { storage } from '@/lib/storage'
import { LoginPage } from '@/components/login-page'
import { Dashboard } from '@/components/dashboard'
import { ClientKeysPage } from '@/components/client-keys-page'
import { useQueryClient } from '@tanstack/react-query'
import { Toaster } from '@/components/ui/sonner'

function App() {
  const [isLoggedIn, setIsLoggedIn] = useState(false)
  const [page, setPage] = useState(window.location.hash)
  const queryClient = useQueryClient()
  useEffect(() => {
    const navigate = () => setPage(window.location.hash)
    window.addEventListener('hashchange', navigate)
    return () => window.removeEventListener('hashchange', navigate)
  }, [])

  useEffect(() => {
    // 检查是否已经有保存的 API Key
    if (storage.getApiKey()) {
      setIsLoggedIn(true)
    }
  }, [])

  const handleLogin = () => {
    setIsLoggedIn(true)
  }

  const handleLogout = () => {
    storage.removeApiKey()
    queryClient.clear()
    setIsLoggedIn(false)
  }

  return (
    <>
      {isLoggedIn ? (
        page === '#client-keys' ? <ClientKeysPage onBack={() => { window.location.hash = '' }} onLogout={handleLogout} />
          : <Dashboard onLogout={handleLogout} />
      ) : (
        <LoginPage onLogin={handleLogin} />
      )}
      <Toaster position="top-right" />
    </>
  )
}

export default App
