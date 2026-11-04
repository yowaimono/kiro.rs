import { cn } from '@/lib/utils'

export function Brand({ className }: { className?: string }) {
  return (
    <div className={cn('brand', className)} aria-label="问高云 Kiro 控制台">
      <img src={`${import.meta.env.BASE_URL}cloud-logo.png`} alt="" width="36" height="26" />
      <span>问高云</span>
      <span className="brand-divider" aria-hidden="true" />
      <span className="brand-product">Kiro 控制台</span>
    </div>
  )
}
