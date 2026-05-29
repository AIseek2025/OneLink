import { tokens } from '../design-tokens';

interface NavItem {
  label: string;
  href: string;
  active?: boolean;
}

interface OlNavBarProps {
  brand?: string;
  items: NavItem[];
  onLogout?: () => void;
}

export function OlNavBar({ brand = 'OneLink', items, onLogout }: OlNavBarProps) {
  return (
    <nav
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: `${tokens.spacing.md} ${tokens.spacing.xl}`,
        background: tokens.color.neutral.bg,
        borderBottom: `1px solid ${tokens.color.neutral.border}`,
        position: 'sticky',
        top: 0,
        zIndex: tokens.zIndex.sticky,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.xl }}>
        <span
          style={{
            color: tokens.color.brand.primary,
            fontWeight: tokens.typography.fontWeight.bold,
            fontSize: tokens.typography.fontSize.xl,
            cursor: 'pointer',
          }}
          onClick={() => (window.location.href = '/')}
        >
          {brand}
        </span>
        <div style={{ display: 'flex', gap: tokens.spacing.lg }}>
          {items.map((item) => (
            <a
              key={item.href}
              href={item.href}
              style={{
                color: item.active ? tokens.color.brand.primary : tokens.color.neutral['text-secondary'],
                textDecoration: 'none',
                fontSize: tokens.typography.fontSize.sm,
                fontWeight: item.active ? tokens.typography.fontWeight.semibold : tokens.typography.fontWeight.medium,
                padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
                borderRadius: tokens.borderRadius.md,
                background: item.active ? tokens.color.brand['primary-light'] : 'transparent',
                transition: `color var(--ol-motion-fast, 150ms) ease-in-out, background var(--ol-motion-fast, 150ms) ease-in-out`,
              }}
            >
              {item.label}
            </a>
          ))}
        </div>
      </div>
      {onLogout && (
        <button
          onClick={onLogout}
          style={{
            background: 'transparent',
            border: `1px solid ${tokens.color.neutral.border}`,
            borderRadius: tokens.borderRadius.md,
            padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
            color: tokens.color.neutral['text-secondary'],
            cursor: 'pointer',
            fontSize: tokens.typography.fontSize.sm,
          }}
        >
          退出
        </button>
      )}
    </nav>
  );
}