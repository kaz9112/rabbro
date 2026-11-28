import { useState, useRef, useEffect } from 'react';
import { FONTS, THEMES } from '../api/commands';
import type { AppSnapshot } from '../types/app';

export type TabKey = 'boq' | 'hd' | 'ahs' | 'ua';

interface Props {
  snapshot: AppSnapshot;
  activeTab: TabKey;
  onTabChange: (tab: TabKey) => void;
  onUpdateMeta: (patch: Record<string, unknown>) => void;
  onOpenProject: () => void;
  onSaveProject: () => void;
  onExportXlsx: () => void;
}

export function Header({
  snapshot,
  activeTab,
  onTabChange,
  onUpdateMeta,
  onOpenProject,
  onSaveProject,
  onExportXlsx,
}: Props) {
  const { state, dirty, hd_count, ov_count, ua_count } = snapshot;
  const [openSettings, setOpenSettings] = useState(false);
  const popRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDocClick = (e: MouseEvent) => {
      if (popRef.current && !popRef.current.contains(e.target as Node)) {
        setOpenSettings(false);
      }
    };
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);

  const tabs: { key: TabKey; label: string; count?: number }[] = [
    { key: 'boq', label: 'Bill of Quantity' },
    { key: 'hd', label: 'Harga Dasar', count: hd_count },
    { key: 'ahs', label: 'Kalkulasi AHS', count: ov_count },
    { key: 'ua', label: 'AHS Buatan', count: ua_count },
  ];

  return (
    <header className="topbar">
      <div className="brand-wrap">
        <span
          className={`dot-status ${dirty ? 'dirty' : ''}`}
          title={dirty ? 'Belum disimpan ke file' : 'Tersimpan'}
        />
        <input
          type="text"
          className="brand-input"
          value={state.title}
          aria-label="Judul proyek"
          onChange={(e) => onUpdateMeta({ title: e.target.value })}
        />
      </div>

      <nav className="top-tabs" role="tablist">
        {tabs.map((t) => (
          <button
            key={t.key}
            role="tab"
            aria-selected={activeTab === t.key}
            onClick={() => onTabChange(t.key)}
          >
            <span>{t.label}</span>
            {t.count ? <span className="tab-count">{t.count}</span> : null}
          </button>
        ))}
      </nav>

      <div className="topbar-right" ref={popRef}>
        <button className="ghost" onClick={onOpenProject}>
          Buka
        </button>
        <button className="ghost" onClick={onSaveProject}>
          Simpan
        </button>
        <button className="pri" onClick={onExportXlsx}>
          Ekspor Excel
        </button>
        <button
          className="ghost"
          onClick={() => setOpenSettings((v) => !v)}
          aria-expanded={openSettings}
          title="Pengaturan"
        >
          ⚙
        </button>

        {openSettings && (
          <div className="popover">
            <label className="field">
              Subjudul Dokumen
              <input
                type="text"
                value={state.sub}
                onChange={(e) => onUpdateMeta({ sub: e.target.value })}
              />
            </label>
            <label className="field">
              Dasar Harga Satuan
              <select
                value={state.basis}
                onChange={(e) => onUpdateMeta({ basis: e.target.value })}
              >
                <option value="h">Total incl. O&amp;P</option>
                <option value="b">Bare cost</option>
              </select>
            </label>
            <label className="field">
              PPN (%)
              <input
                type="number"
                min={0}
                max={100}
                step={0.5}
                value={state.ppn}
                onChange={(e) =>
                  onUpdateMeta({ ppn: parseFloat(e.target.value) || 0 })
                }
              />
            </label>
            <label className="field">
              Tema Warna
              <select
                value={state.theme}
                onChange={(e) => onUpdateMeta({ theme: e.target.value })}
              >
                {Object.entries(THEMES).map(([k, label]) => (
                  <option key={k} value={k}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              Font Aplikasi &amp; Excel
              <select
                value={state.font}
                onChange={(e) => onUpdateMeta({ font: e.target.value })}
              >
                {Object.entries(FONTS).map(([k, f]) => (
                  <option key={k} value={k}>
                    {f.label}
                  </option>
                ))}
              </select>
            </label>
          </div>
        )}
      </div>
    </header>
  );
}