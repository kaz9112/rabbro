import { invoke } from '@tauri-apps/api/core';
import type {
  AhsEditorView,
  AppSnapshot,
  HdView,
  OpenProjectResult,
  SaveUserAhsPayload,
  UserAhsListItem,
} from '../types/app';

export const api = {
  getSnapshot: () => invoke<AppSnapshot>('get_snapshot'),
  updateMeta: (payload: Record<string, unknown>) =>
    invoke<AppSnapshot>('update_meta', { payload }),
  addBoqLine: (code?: string) =>
    invoke<AppSnapshot>('add_boq_line', { code: code ?? null }),
  duplicateBoqLine: (id: number) =>
    invoke<AppSnapshot>('duplicate_boq_line', { id }),
  updateBoqLine: (payload: Record<string, unknown>) =>
    invoke<AppSnapshot>('update_boq_line', { payload }),
  deleteBoqLine: (id: number) =>
    invoke<AppSnapshot>('delete_boq_line', { id }),
  clearBoq: () => invoke<AppSnapshot>('clear_boq'),

  getHdView: () => invoke<HdView>('get_hd_view'),
  setHdOverride: (r: number, value: number | null) =>
    invoke<AppSnapshot>('set_hd_override', { r, value }),
  resetAllHd: () => invoke<AppSnapshot>('reset_all_hd'),
  addUserHd: (payload: {
    cat: string;
    k: string;
    n: string;
    u: string;
    v: number;
    c: string;
  }) => invoke<AppSnapshot>('add_user_hd', { payload }),
  updateUserHd: (r: number, n?: string, u?: string) =>
    invoke<AppSnapshot>('update_user_hd', { r, n: n ?? null, u: u ?? null }),
  deleteUserHd: (r: number) => invoke<AppSnapshot>('delete_user_hd', { r }),

  getAhsEditorView: (idx: number) =>
    invoke<AhsEditorView>('get_ahs_editor_view', { idx }),
  commitAhsCell: (idx: number, addr: string, text: string) =>
    invoke<AppSnapshot>('commit_ahs_cell', { idx, addr, text }),
  resetAhsOverrides: (idx: number | null) =>
    invoke<AppSnapshot>('reset_ahs_overrides', { idx }),
  relinkHdInAhs: (idx: number, addr: string, newHdText: string) =>
    invoke<AppSnapshot>('relink_hd_in_ahs', { idx, addr, newHdText }),
  getUserAhsList: () => invoke<UserAhsListItem[]>('get_user_ahs_list'),
  saveUserAhs: (payload: SaveUserAhsPayload) =>
    invoke<AppSnapshot>('save_user_ahs', { payload }),
  deleteUserAhs: (id: number) => invoke<AppSnapshot>('delete_user_ahs', { id }),

  saveProjectDialog: () =>
    invoke<AppSnapshot | null>('save_project_dialog'),
  openProjectDialog: () =>
    invoke<OpenProjectResult | null>('open_project_dialog'),
  exportXlsxDialog: () => invoke<boolean>('export_xlsx_dialog'),
  exportCsvDialog: () => invoke<boolean>('export_csv_dialog'),
  getBoqTsv: () => invoke<string>('get_boq_tsv'),
};

export const rp = (n: number) =>
  new Intl.NumberFormat('id-ID', { maximumFractionDigits: 0 }).format(
    Math.round(n || 0)
  );

export const fmt = (n: number) =>
  new Intl.NumberFormat('id-ID', { maximumFractionDigits: 2 }).format(n || 0);

export const f4 = (n: number) =>
  new Intl.NumberFormat('id-ID', { maximumFractionDigits: 4 }).format(n || 0);

export const pctDiff = (a: number, b: number) => {
  if (!a || Math.abs(a - b) < 1e-9) return null;
  const p = ((b - a) / a) * 100;
  return {
    pct: p,
    text: `${p > 0 ? '+' : ''}${p.toFixed(1).replace('.', ',')}%`,
  };
};

export const THEMES: Record<string, string> = {
  exist: 'Hijau Emerald',
  light: 'Abu Terang',
  dark: 'Grafit Gelap',
  blue: 'Biru Cobalt',
  red: 'Merah Crimson',
};

export const FONTS: Record<string, { label: string; sans: string; mono?: string }> = {
  exist: {
    label: 'IBM Plex Sans (Bawaan)',
    sans: '"Inter", "IBM Plex Sans", system-ui, -apple-system, sans-serif',
    mono: '"JetBrains Mono", "IBM Plex Mono", ui-monospace, monospace',
  },
  arial: { label: 'Arial', sans: 'Arial, Helvetica, sans-serif' },
  calibri: {
    label: 'Calibri',
    sans: 'Calibri, Carlito, "Segoe UI", Arial, sans-serif',
  },
  tahoma: { label: 'Tahoma', sans: 'Tahoma, Verdana, Geneva, sans-serif' },
  times: { label: 'Times New Roman', sans: '"Times New Roman", Times, serif' },
};