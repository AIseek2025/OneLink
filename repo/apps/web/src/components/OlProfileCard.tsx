import { tokens } from '../design-tokens';

interface ProfileFact {
  fact_type: string;
  value: string;
  confidence: number;
}

interface ProfileTraits {
  interest_tags: string[];
  connection_goal_tags: string[];
  location_label: string | null;
  communication_preferences: string[];
}

interface OlProfileCardProps {
  displayName?: string;
  avatarUrl?: string;
  cityLocation?: string;
  languages?: string[];
  facts?: ProfileFact[];
  traits?: ProfileTraits;
  completionRate?: number;
  missingDimensions?: string[];
  onEdit?: () => void;
}

export function OlProfileCard({
  displayName,
  avatarUrl,
  cityLocation,
  languages,
  traits,
  completionRate,
  missingDimensions,
  onEdit,
}: OlProfileCardProps) {
  return (
    <div
      style={{
        background: tokens.color.neutral.bg,
        borderRadius: tokens.borderRadius.lg,
        padding: tokens.spacing.xl,
        boxShadow: tokens.shadow.sm,
      }}
    >
      {completionRate !== undefined && (
        <div
          style={{
            marginBottom: tokens.spacing.lg,
            padding: tokens.spacing.md,
            background: tokens.color.neutral.surface,
            borderRadius: tokens.borderRadius.lg,
          }}
        >
          <p
            style={{
              color: tokens.color.neutral['text-primary'],
              fontWeight: tokens.typography.fontWeight.medium,
            }}
          >
            完成度: {Math.round(completionRate * 100)}%
          </p>
          {missingDimensions && missingDimensions.length > 0 && (
            <p
              style={{
                color: tokens.color.neutral['text-secondary'],
                fontSize: tokens.typography.fontSize.sm,
              }}
            >
              待完善: {missingDimensions.join(', ')}
            </p>
          )}
        </div>
      )}

      <div style={{ marginBottom: tokens.spacing.lg }}>
        {avatarUrl && (
          <img
            src={avatarUrl}
            alt={displayName ?? 'avatar'}
            style={{
              width: 48,
              height: 48,
              borderRadius: tokens.borderRadius.full,
              objectFit: 'cover',
              marginBottom: tokens.spacing.sm,
            }}
          />
        )}
        <p
          style={{
            color: tokens.color.neutral['text-primary'],
            fontWeight: tokens.typography.fontWeight.semibold,
            fontSize: tokens.typography.fontSize.lg,
          }}
        >
          {displayName || '(未填写)'}
        </p>
        {cityLocation && (
          <p style={{ color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>
            {cityLocation}
          </p>
        )}
        {languages && languages.length > 0 && (
          <p style={{ color: tokens.color.neutral['text-secondary'], fontSize: tokens.typography.fontSize.sm }}>
            语言: {languages.join(', ')}
          </p>
        )}
      </div>

      {traits && (
        <div style={{ marginBottom: tokens.spacing.lg }}>
          {traits.interest_tags.length > 0 && (
            <div style={{ marginBottom: tokens.spacing.sm }}>
              <span style={{ fontWeight: tokens.typography.fontWeight.medium, fontSize: tokens.typography.fontSize.sm }}>兴趣: </span>
              {traits.interest_tags.map((tag) => (
                <span
                  key={tag}
                  style={{
                    display: 'inline-block',
                    padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`,
                    background: tokens.color.brand['primary-light'],
                    color: tokens.color.brand.primary,
                    borderRadius: tokens.borderRadius.full,
                    fontSize: tokens.typography.fontSize.xs,
                    marginRight: tokens.spacing.xs,
                  }}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
          {traits.connection_goal_tags.length > 0 && (
            <div style={{ marginBottom: tokens.spacing.sm }}>
              <span style={{ fontWeight: tokens.typography.fontWeight.medium, fontSize: tokens.typography.fontSize.sm }}>目标: </span>
              {traits.connection_goal_tags.map((tag) => (
                <span
                  key={tag}
                  style={{
                    display: 'inline-block',
                    padding: `${tokens.spacing.xs} ${tokens.spacing.sm}`,
                    background: tokens.color.brand['primary-light'],
                    color: tokens.color.brand.primary,
                    borderRadius: tokens.borderRadius.full,
                    fontSize: tokens.typography.fontSize.xs,
                    marginRight: tokens.spacing.xs,
                  }}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
          {traits.communication_preferences.length > 0 && (
            <div>
              <span style={{ fontWeight: tokens.typography.fontWeight.medium, fontSize: tokens.typography.fontSize.sm }}>沟通偏好: </span>
              {traits.communication_preferences.join(', ')}
            </div>
          )}
        </div>
      )}

      {onEdit && (
        <button
          onClick={onEdit}
          style={{
            padding: `${tokens.spacing.sm} ${tokens.spacing.lg}`,
            background: tokens.color.brand.primary,
            color: '#FFFFFF',
            border: 'none',
            borderRadius: tokens.borderRadius.md,
            cursor: 'pointer',
            fontWeight: tokens.typography.fontWeight.semibold,
            fontSize: tokens.typography.fontSize.sm,
          }}
        >
          编辑资料
        </button>
      )}
    </div>
  );
}