import logoUrl from '@/assets/logo.svg'

export function Logo({ size = 24, className }: { size?: number; className?: string }) {
  return (
    <img
      src={logoUrl}
      width={size}
      height={size}
      alt=""
      aria-hidden="true"
      className={className}
    />
  )
}
