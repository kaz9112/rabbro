export type TabKey = 'boq' | 'hd' | 'ahs' | 'ua';

interface Props {
  active: TabKey;
  onChange: (tab: TabKey) => void;
  hdCount: number;
  ovCount: number;
  uaCount: number;
}

export function TabBar({ active, onChange, hdCount, ovCount, uaCount }: Props) {
  const tabs: { key: TabKey; label: string; count?: number }[] = [
    { key: 'boq', label: 'Bill of Quantity' },
    { key: 'hd', label: 'Revisi Harga Dasar', count: hdCount },
    { key: 'ahs', label: 'Edit Kalkulasi AHS', count: ovCount },
    { key: 'ua', label: 'AHS Buatan Sendiri', count: uaCount },
  ];

  return (
    <div className="nav-wrap">
      <div className="seg-tabs" role="tablist">
        {tabs.map((t) => (
          <button
            key={t.key}
            role="tab"
            aria-selected={active === t.key}
            onClick={() => onChange(t.key)}
          >
            <span>{t.label}</span>
            {t.count ? <span className="badge">{t.count}</span> : null}
          </button>
        ))}
      </div>
    </div>
  );
}