import { useId } from 'react';
import { useTranslation } from 'react-i18next';

// The app icon itself, drawn inline from the same geometry as public/nexus-mark.svg
// (cropped to its 824px tile), so the mark inside the app matches the Dock and
// Finder icon exactly. Fixed paper and ink colours: the icon does not re-theme.
export function NexusMark({ size = 28 }: { size?: number }) {
  const paper = useId();
  return <svg className="nexus-mark" width={size} height={size} viewBox="100 100 824 824" aria-hidden="true">
    <defs><linearGradient id={paper} x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor="#F7F5EE" /><stop offset="1" stopColor="#E6E1D3" /></linearGradient></defs>
    <rect x="100" y="100" width="824" height="824" rx="186" fill={`url(#${paper})`} />
    <rect x="100" y="740" width="824" height="3" fill="#191A1C" opacity="0.14" />
    <rect x="104" y="104" width="816" height="816" rx="182" fill="none" stroke="#191A1C" strokeOpacity="0.14" strokeWidth="8" />
    <circle cx="684" cy="362" r="116" fill="#2E44B5" opacity="0.86" />
    <path fill="#1B1C1F" style={{ mixBlendMode: 'multiply' }} d="M300 282H322V742H300ZM300 282H404L728 742H624ZM706 282H728V742H706ZM256 282H404V292H256ZM660 282H774V292H660ZM254 732H368V742H254Z" />
  </svg>;
}

export function NexusIdentity() {
  const { t } = useTranslation('common');
  return <div className="brand" role="img" aria-label={t('productName')}>
    <NexusMark />
    <span className="wordmark" lang="en-US" aria-hidden="true">{t('brandName')}</span>
  </div>;
}
