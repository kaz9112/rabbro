import { useCallback, useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm } from '@tauri-apps/plugin-dialog';
import { api, FONTS } from './api/commands';
import { Header, type TabKey } from './components/Header';
import { AhsEditorView } from './features/ahs-editor/AhsEditorView';
import { BoqView } from './features/boq/BoqView';
import { HargaDasarView } from './features/harga-dasar/HargaDasarView';
import { UserAhsView } from './features/user-ahs/UserAhsView';
import type { AppSnapshot } from './types/app';

export default function App() {
  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [tab, setTab] = useState<TabKey>('boq');
  const [selectedAhsIdx, setSelectedAhsIdx] = useState(0);
  const [deriveTarget, setDeriveTarget] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  const dirtyRef = useRef(false);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToastMsg(null), 2400);
  }, []);

  useEffect(() => {
    api
      .getSnapshot()
      .then(setSnapshot)
      .catch((e) => showToast(`Gagal memuat: ${String(e)}`));
  }, [showToast]);

  useEffect(() => {
    if (!snapshot) return;
    dirtyRef.current = snapshot.dirty;

    const root = document.documentElement;
    if (snapshot.state.theme === 'exist') {
      root.removeAttribute('data-th');
    } else {
      root.setAttribute('data-th', snapshot.state.theme);
    }

    const f = FONTS[snapshot.state.font] ?? FONTS.exist;
    root.style.setProperty('--sans', f.sans);
    root.style.setProperty('--mono', f.mono ?? f.sans);

    try {
      getCurrentWindow()
        .setTitle(snapshot.state.title || 'RabBro')
        .catch(() => {});
    } catch (_) {}
  }, [snapshot]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    try {
      const win = getCurrentWindow();
      win
        .onCloseRequested(async (event) => {
          if (!dirtyRef.current) return;
          event.preventDefault();
          const ok = await confirm(
            'Ada perubahan yang belum disimpan ke file proyek (.json). Tetap keluar?',
            { title: 'Konfirmasi Keluar', kind: 'warning' }
          );
          if (ok) {
            dirtyRef.current = false;
            await win.destroy();
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch(() => {});
    } catch (_) {}

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleUpdateMeta = async (patch: Record<string, unknown>) => {
    try {
      setSnapshot(await api.updateMeta(patch));
    } catch (e) {
      showToast(String(e));
    }
  };

  const handleSaveProject = async () => {
    try {
      const res = await api.saveProjectDialog();
      if (res) {
        setSnapshot(res);
        showToast('Proyek tersimpan');
      }
    } catch (e) {
      showToast(String(e));
    }
  };

  const handleOpenProject = async () => {
    try {
      if (snapshot?.dirty) {
        const ok = await confirm(
          'Pekerjaan saat ini belum disimpan ke file. Tetap buka file lain?',
          { title: 'Buka Proyek', kind: 'warning' }
        );
        if (!ok) return;
      }
      const res = await api.openProjectDialog();
      if (res) {
        setSnapshot(res.snapshot);
        setTab('boq');
        showToast(`Dibuka: ${res.filename}`);
      }
    } catch (e) {
      showToast(String(e));
    }
  };

  const handleExportXlsx = async () => {
    try {
      if (await api.exportXlsxDialog()) showToast('Excel (.xlsx) diekspor');
    } catch (e) {
      showToast(String(e));
    }
  };

  const handleOpenInEditor = (code: string) => {
    if (!snapshot) return;
    const ahsOnly = snapshot.catalog.filter((c) => !c.m);
    const ix = ahsOnly.findIndex((c) => c.k === code);
    setSelectedAhsIdx(ix >= 0 ? ix : 0);
    setTab('ahs');
  };

  if (!snapshot) {
    return (
      <div className="app-shell" style={{ justifyContent: 'center', alignItems: 'center' }}>
        Memuat...
      </div>
    );
  }

  return (
    <div className="app-shell">
      <Header
        snapshot={snapshot}
        activeTab={tab}
        onTabChange={setTab}
        onUpdateMeta={handleUpdateMeta}
        onOpenProject={handleOpenProject}
        onSaveProject={handleSaveProject}
        onExportXlsx={handleExportXlsx}
      />

      <main className="workspace">
        {tab === 'boq' && (
          <BoqView
            snapshot={snapshot}
            onSnapshot={setSnapshot}
            onToast={showToast}
            onUpdateMeta={handleUpdateMeta}
          />
        )}
        {tab === 'hd' && (
          <HargaDasarView
            snapshot={snapshot}
            onSnapshot={setSnapshot}
            onToast={showToast}
          />
        )}
        {tab === 'ahs' && (
          <AhsEditorView
            snapshot={snapshot}
            selectedIdx={selectedAhsIdx}
            onSelectIdx={setSelectedAhsIdx}
            onSnapshot={setSnapshot}
            onToast={showToast}
            onDeriveAhs={(srcCode) => {
              setDeriveTarget(srcCode);
              setTab('ua');
            }}
          />
        )}
        {tab === 'ua' && (
          <UserAhsView
            snapshot={snapshot}
            deriveTargetCode={deriveTarget}
            onClearDeriveTarget={() => setDeriveTarget(null)}
            onSnapshot={setSnapshot}
            onOpenInEditor={handleOpenInEditor}
            onToast={showToast}
          />
        )}
      </main>

      <footer className="app-footer">
        <span>Ver. 0.1</span>
        <span>
          Kalimantan Industrial Park Indonesia
          Mangkupadi
        </span>
      </footer>

      {toastMsg && (
        <div className="toast" role="status">
          {toastMsg}
        </div>
      )}
    </div>
  );
}