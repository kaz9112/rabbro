import { useMemo, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { api, fmt, rp } from '../../api/commands';
import type { AppSnapshot, BoqRenderLine, CatalogEntry } from '../../types/app';

interface Props {
  snapshot: AppSnapshot;
  onSnapshot: (snap: AppSnapshot) => void;
  onToast: (msg: string) => void;
  onUpdateMeta: (patch: Record<string, unknown>) => void;
}

export function BoqView({ snapshot, onSnapshot, onToast, onUpdateMeta }: Props) {
  const [query, setQuery] = useState('');
  const [divFilter, setDivFilter] = useState('');
  const [armClear, setArmClear] = useState(false);
  const [showMeta, setShowMeta] = useState(false);

  const { state, divs = {}, catalog = [], boq, default_filename = '' } = snapshot;
  const groups = boq?.groups ?? [];
  const lineCount = boq?.line_count ?? 0;
  const subtotal = boq?.subtotal ?? 0;
  const tax = boq?.tax ?? 0;
  const grandTotal = boq?.grand_total ?? 0;

  const filteredCatalog = useMemo(() => {
    const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
    return catalog.filter((it) => {
      if (divFilter && it.dv !== divFilter) return false;
      const hay = `${it.k} ${it.u} ${it.g}`.toLowerCase();
      return words.every((w) => hay.includes(w));
    });
  }, [catalog, query, divFilter]);

  const subdivSuggestions = useMemo(() => {
    const map: Record<string, string[]> = {};
    for (const it of catalog) {
      if (!map[it.dv]) map[it.dv] = [];
      if (!map[it.dv].includes(it.g)) map[it.dv].push(it.g);
    }
    return map;
  }, [catalog]);

  const handleAddItem = async (it: CatalogEntry) => {
    try {
      onSnapshot(await api.addBoqLine(it.k));
      onToast(`+ ${it.k}`);
    } catch (e) {
      onToast(String(e));
    }
  };

  const handleClear = async () => {
    if (!state.lines.length) return;
    if (!armClear) {
      setArmClear(true);
      setTimeout(() => setArmClear(false), 3000);
      return;
    }
    setArmClear(false);
    onSnapshot(await api.clearBoq());
    onToast('BoQ dikosongkan');
  };

  const handleCopyTsv = async () => {
    try {
      const tsv = await api.getBoqTsv();
      try {
        await writeText(tsv);
      } catch {
        await navigator.clipboard.writeText(tsv);
      }
      onToast('Tabel disalin ke clipboard');
    } catch (e) {
      onToast(`Gagal menyalin: ${String(e)}`);
    }
  };

  const handleExportCsv = async () => {
    try {
      if (await api.exportCsvDialog()) onToast('CSV disimpan');
    } catch (e) {
      onToast(String(e));
    }
  };

  let rowNo = 0;

  const renderLineRow = (rl: BoqRenderLine) => {
    const { line, rate, amount } = rl;
    rowNo += 1;
    const isEditablePrice = line.custom || line.man;

    return (
      <tr key={line.id}>
        <td style={{ color: 'var(--mute)', fontSize: 11.5 }}>{rowNo}</td>
        <td>
          {line.custom ? (
            <>
              <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                <input
                  type="text"
                  className="cell-in"
                  style={{ fontWeight: 500 }}
                  value={line.u}
                  placeholder="Uraian pekerjaan custom"
                  onChange={(e) =>
                    api
                      .updateBoqLine({ id: line.id, u: e.target.value })
                      .then(onSnapshot)
                  }
                />
                <span className="tag">CUSTOM</span>
              </div>
              <div style={{ display: 'flex', gap: 6, marginTop: 2 }}>
                <select
                  className="cell-in sub"
                  style={{ width: 'auto', maxWidth: 220 }}
                  value={line.dv}
                  onChange={(e) =>
                    api
                      .updateBoqLine({ id: line.id, dv: e.target.value })
                      .then(onSnapshot)
                  }
                >
                  <option value="">Pekerjaan Tambahan</option>
                  {Object.entries(divs)
                    .sort(([a], [b]) => a.localeCompare(b))
                    .map(([k, v]) => (
                      <option key={k} value={k}>
                        Divisi {k} {v}
                      </option>
                    ))}
                </select>
                <input
                  type="text"
                  className="cell-in sub"
                  list={`boq-sdl-${line.dv}`}
                  disabled={!line.dv}
                  value={line.g}
                  placeholder={line.dv ? 'Subdivisi' : ''}
                  onChange={(e) =>
                    api
                      .updateBoqLine({ id: line.id, g: e.target.value })
                      .then(onSnapshot)
                  }
                />
              </div>
            </>
          ) : (
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span className="code-txt">{line.k}</span>
                {line.man && <span className="tag">MANUAL</span>}
              </div>
              <div>{line.u}</div>
            </div>
          )}
          <input
            type="text"
            className="cell-in sub"
            value={line.note}
            placeholder="Tambah catatan..."
            onChange={(e) =>
              api
                .updateBoqLine({ id: line.id, note: e.target.value })
                .then(onSnapshot)
            }
          />
        </td>
        <td>
          {isEditablePrice ? (
            <input
              type="text"
              className="cell-in"
              value={line.s}
              onChange={(e) =>
                api
                  .updateBoqLine({ id: line.id, s: e.target.value })
                  .then(onSnapshot)
              }
            />
          ) : (
            <span style={{ color: 'var(--mute)' }}>{line.s}</span>
          )}
        </td>
        <td>
          <input
            type="number"
            min={0}
            step="any"
            className="cell-in num"
            value={line.qty || ''}
            placeholder="0"
            onChange={(e) =>
              api
                .updateBoqLine({
                  id: line.id,
                  qty: parseFloat(e.target.value) || 0,
                })
                .then(onSnapshot)
            }
          />
        </td>
        <td className="num">
          {isEditablePrice ? (
            <input
              type="number"
              min={0}
              step="any"
              className="cell-in num"
              value={line.price || ''}
              placeholder="0"
              onChange={(e) =>
                api
                  .updateBoqLine({
                    id: line.id,
                    price: parseFloat(e.target.value) || 0,
                  })
                  .then(onSnapshot)
              }
            />
          ) : (
            rp(rate)
          )}
        </td>
        <td className="num" style={{ fontWeight: 500 }}>
          {rp(amount)}
        </td>
        <td className="row-actions">
          <button
            className="icon-btn"
            title="Gandakan"
            onClick={() => api.duplicateBoqLine(line.id).then(onSnapshot)}
          >
            ⧉
          </button>
          <button
            className="icon-btn danger"
            title="Hapus"
            onClick={() => api.deleteBoqLine(line.id).then(onSnapshot)}
          >
            ✕
          </button>
        </td>
      </tr>
    );
  };

  let lastGroup = '';

  return (
    <div className="boq-split">
      {/* Left: Compact Catalog */}
      <aside className="catalog-pane">
        <div className="catalog-head">
          <div className="catalog-head-top">
            <span>Katalog AHS</span>
            <span>
              {filteredCatalog.length}/{catalog.length}
            </span>
          </div>
          <select
            value={divFilter}
            onChange={(e) => setDivFilter(e.target.value)}
            aria-label="Filter divisi"
          >
            <option value="">Semua divisi</option>
            {Object.entries(divs)
              .sort(([a], [b]) => a.localeCompare(b))
              .map(([k, v]) => (
                <option key={k} value={k}>
                  Div {k} — {v}
                </option>
              ))}
          </select>
          <input
            type="text"
            placeholder="Cari kode / uraian..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>

        <div className="catalog-scroll">
          {filteredCatalog.length === 0 ? (
            <div className="empty">Tidak ada item cocok.</div>
          ) : (
            filteredCatalog.map((it) => {
              const showGroup = it.g !== lastGroup;
              lastGroup = it.g;
              const changed = Math.abs(it.price0 - it.price) > 0.5;
              return (
                <div key={it.k}>
                  {showGroup && <div className="cat-group">{it.g}</div>}
                  <div
                    className="cat-row"
                    onClick={() => handleAddItem(it)}
                    title="Klik untuk menambahkan ke BoQ"
                  >
                    <div className="cat-row-main">
                      <div className="cat-row-top">
                        <span>
                          {it.k}
                          {it.usr !== null && (
                            <span className="tag soft">BUATAN</span>
                          )}
                        </span>
                        <span>
                          {it.m ? (
                            <span className="tag">MANUAL</span>
                          ) : (
                            <>
                              {changed && (
                                <span className="old-price">{rp(it.price0)}</span>
                              )}
                              Rp {rp(it.price)}/{it.s}
                            </>
                          )}
                        </span>
                      </div>
                      <div className="cat-row-name">{it.u}</div>
                    </div>
                    <button
                      className="cat-add-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleAddItem(it);
                      }}
                    >
                      +
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </aside>

      {/* Right: Spreadsheet BoQ */}
      <section className="boq-pane">
        <div className="pane-toolbar">
          <button
            className="meta-pill"
            onClick={() => setShowMeta((v) => !v)}
            title="Klik untuk edit detail proyek & dokumen"
          >
            <span>
              {state.proj || 'Proyek Baru'} · Rev {state.rev || '0'} ·{' '}
              {state.date || '—'}
            </span>
            <span style={{ color: 'var(--mute)' }}>{showMeta ? '▴' : '▾'}</span>
          </button>

          <div style={{ display: 'flex', gap: 6 }}>
            <button onClick={() => api.addBoqLine().then(onSnapshot)}>
              + Item Custom
            </button>
            <button className="ghost" onClick={handleClear}>
              {armClear ? 'Klik lagi untuk hapus' : 'Kosongkan'}
            </button>
          </div>
        </div>

        {showMeta && (
          <div className="meta-drawer">
            <label className="field">
              Proyek
              <input
                type="text"
                value={state.proj}
                onChange={(e) => onUpdateMeta({ proj: e.target.value })}
              />
            </label>
            <label className="field">
              Perusahaan
              <input
                type="text"
                value={state.company}
                onChange={(e) => onUpdateMeta({ company: e.target.value })}
              />
            </label>
            <label className="field">
              Dibuat oleh
              <input
                type="text"
                value={state.by}
                onChange={(e) => onUpdateMeta({ by: e.target.value })}
              />
            </label>
            <label className="field">
              Tanggal
              <input
                type="date"
                value={state.date}
                onChange={(e) => onUpdateMeta({ date: e.target.value })}
              />
            </label>
            <label className="field">
              Revisi No
              <input
                type="text"
                value={state.rev}
                onChange={(e) => onUpdateMeta({ rev: e.target.value })}
              />
            </label>
            <label className="field">
              Nama File Ekspor
              <input
                type="text"
                value={state.fname}
                placeholder={default_filename}
                onChange={(e) => onUpdateMeta({ fname: e.target.value })}
              />
            </label>
          </div>
        )}

        <div className="sheet-scroll">
          <table className="sheet" style={{ minWidth: 700 }}>
            <thead>
              <tr>
                <th style={{ width: 38 }}>No</th>
                <th>Uraian &amp; Catatan</th>
                <th style={{ width: 68 }}>Sat</th>
                <th className="n" style={{ width: 96 }}>
                  Volume
                </th>
                <th className="n" style={{ width: 124 }}>
                  Harga Satuan
                </th>
                <th className="n" style={{ width: 132 }}>
                  Jumlah (Rp)
                </th>
                <th style={{ width: 58 }}></th>
              </tr>
            </thead>
            <tbody>
              {lineCount === 0 || groups.length === 0 ? (
                <tr>
                  <td colSpan={7}>
                    <div className="empty">
                      BoQ kosong. Klik item di katalog sebelah kiri untuk
                      menambahkan pekerjaan.
                    </div>
                  </td>
                </tr>
              ) : (
                groups.flatMap((grp) => [
                  <tr key={`dv-${grp.key}`} className="row-dv">
                    <td colSpan={5}>{grp.label}</td>
                    <td className="num">{rp(grp.subtotal)}</td>
                    <td></td>
                  </tr>,
                  ...(grp.subgroups ?? []).flatMap((sub) => [
                    <tr key={`sd-${grp.key}-${sub.full_g}`} className="row-sd">
                      <td colSpan={5}>{sub.full_g}</td>
                      <td className="num">{rp(sub.subtotal)}</td>
                      <td></td>
                    </tr>,
                    ...(sub.lines ?? []).map(renderLineRow),
                  ]),
                  ...(grp.direct_lines ?? []).map(renderLineRow),
                ])
              )}
            </tbody>
          </table>
        </div>

        {/* Pinned Totals Bar */}
        <div className="totals-bar">
          <div className="totals-actions">
            <button onClick={handleExportCsv}>CSV</button>
            <button onClick={handleCopyTsv}>Salin Tabel</button>
          </div>
          <div className="totals-figures">
            <div className="fig-item">
              <span>Subtotal ({lineCount})</span>
              <strong>Rp {rp(subtotal)}</strong>
            </div>
            <div className="fig-item">
              <span>PPN {fmt(state.ppn)}%</span>
              <strong>Rp {rp(tax)}</strong>
            </div>
            <div className="fig-item fig-grand">
              <span>Total RAB</span>
              <strong>Rp {rp(grandTotal)}</strong>
            </div>
          </div>
        </div>

        <div hidden>
          {Object.entries(subdivSuggestions).map(([dv, list]) => (
            <datalist key={dv} id={`boq-sdl-${dv}`}>
              {list.map((g) => (
                <option key={g} value={g} />
              ))}
            </datalist>
          ))}
        </div>
      </section>
    </div>
  );
}