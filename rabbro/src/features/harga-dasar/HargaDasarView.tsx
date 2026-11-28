import { useEffect, useState } from 'react';
import { api, fmt, pctDiff, rp } from '../../api/commands';
import type { AppSnapshot, HdView } from '../../types/app';

interface Props {
  snapshot: AppSnapshot;
  onSnapshot: (snap: AppSnapshot) => void;
  onToast: (msg: string) => void;
}

const CATS = [
  ['all', 'Semua'],
  ['L', 'Upah (L)'],
  ['M', 'Bahan (M)'],
  ['E', 'Peralatan (E)'],
  ['P', 'Parameter (P)'],
];

export function HargaDasarView({ snapshot, onSnapshot, onToast }: Props) {
  const [view, setView] = useState<HdView | null>(null);
  const [cat, setCat] = useState('all');
  const [query, setQuery] = useState('');
  const [showAdd, setShowAdd] = useState(false);
  const [armReset, setArmReset] = useState(false);
  const [armDel, setArmDel] = useState<number | null>(null);

  const [newCat, setNewCat] = useState('M');
  const [newCode, setNewCode] = useState('');
  const [newUnit, setNewUnit] = useState('');
  const [newName, setNewName] = useState('');
  const [newPrice, setNewPrice] = useState('');
  const [newComment, setNewComment] = useState('');

  const refresh = () => {
    api.getHdView().then(setView).catch((e) => onToast(String(e)));
  };

  useEffect(() => {
    refresh();
  }, [snapshot]);

  const nextCode = (c: string) => {
    if (!view) return `${c}.01`;
    let m = 0;
    for (const x of view.rows) {
      if (x.k.startsWith(`${c}.`)) {
        const n = parseInt(x.k.slice(2), 10);
        if (!isNaN(n) && n > m) m = n;
      }
    }
    return `${c}.${String(m + 1).padStart(2, '0')}`;
  };

  const openNewForm = () => {
    setNewCat('M');
    setNewCode(nextCode('M'));
    setNewUnit('');
    setNewName('');
    setNewPrice('');
    setNewComment('');
    setShowAdd(true);
  };

  const handleAddUserHd = async () => {
    try {
      const snap = await api.addUserHd({
        cat: newCat,
        k: newCode,
        n: newName,
        u: newUnit,
        v: parseFloat(newPrice) || 0,
        c: newComment,
      });
      onSnapshot(snap);
      setShowAdd(false);
      setQuery(newCode);
      setCat('all');
      onToast(`Item harga dasar ditambahkan: ${newCode}`);
    } catch (e) {
      onToast(String(e));
    }
  };

  const handleResetAll = async () => {
    if (!view?.hd_count) return;
    if (!armReset) {
      setArmReset(true);
      setTimeout(() => setArmReset(false), 3000);
      return;
    }
    setArmReset(false);
    onSnapshot(await api.resetAllHd());
    onToast('Semua harga dasar dikembalikan ke awal');
  };

  const handleDeleteUserHd = async (r: number) => {
    if (armDel !== r) {
      setArmDel(r);
      setTimeout(() => setArmDel((cur) => (cur === r ? null : cur)), 3000);
      return;
    }
    try {
      setArmDel(null);
      onSnapshot(await api.deleteUserHd(r));
      onToast('Item harga dasar dihapus');
    } catch (e) {
      onToast(String(e));
    }
  };

  if (!view) return <div className="card">Memuat Harga Dasar...</div>;

  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const filtered = view.rows.filter(
    (x) =>
      (cat === 'all' || x.k.startsWith(cat)) &&
      words.every((w) => `${x.k} ${x.n}`.toLowerCase().includes(w))
  );

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <section className="card">
        <div className="card-hd">
          <h2>Harga Dasar Analisa Harga Satuan</h2>
          <div style={{ display: 'flex', gap: 8 }}>
            <button className="pri" onClick={openNewForm}>
              + Tambah Item
            </button>
            <button onClick={handleResetAll}>
              {armReset ? 'Klik lagi untuk reset semua' : 'Reset ke Awal'}
            </button>
          </div>
        </div>

        {showAdd && (
          <div className="subpanel">
            <div className="form-grid">
              <label className="field">
                Kategori
                <select
                  value={newCat}
                  onChange={(e) => {
                    setNewCat(e.target.value);
                    setNewCode(nextCode(e.target.value));
                  }}
                >
                  <option value="L">Upah (L)</option>
                  <option value="M">Bahan (M)</option>
                  <option value="E">Alat (E)</option>
                </select>
              </label>
              <label className="field">
                Kode
                <input
                  type="text"
                  value={newCode}
                  onChange={(e) => setNewCode(e.target.value)}
                />
              </label>
              <label className="field">
                Satuan
                <input
                  type="text"
                  placeholder="kg, m3, OH, jam"
                  value={newUnit}
                  onChange={(e) => setNewUnit(e.target.value)}
                />
              </label>
              <label className="field" style={{ gridColumn: '1 / -1' }}>
                Uraian
                <input
                  type="text"
                  placeholder="mis. Semen PCC 40 kg"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                />
              </label>
              <label className="field">
                Harga (Rp)
                <input
                  type="number"
                  min={0}
                  step="any"
                  value={newPrice}
                  onChange={(e) => setNewPrice(e.target.value)}
                />
              </label>
              <label className="field">
                Keterangan (opsional)
                <input
                  type="text"
                  value={newComment}
                  onChange={(e) => setNewComment(e.target.value)}
                />
              </label>
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <button className="pri" onClick={handleAddUserHd}>
                Simpan Item
              </button>
              <button onClick={() => setShowAdd(false)}>Batal</button>
            </div>
          </div>
        )}

        <div className="summary-banner">
          {view.hd_count > 0 ? (
            <>
              <b>{view.hd_count}</b> harga dasar direvisi —{' '}
              <b>{view.impacted.length}</b> dari {view.total_ahs} harga satuan
              AHS berubah.
            </>
          ) : (
            'Belum ada revisi. Semua harga memakai asumsi dasar.'
          )}
        </div>

        <div className="filter-bar">
          <input
            type="text"
            style={{ flex: 1, minWidth: 220 }}
            placeholder="Cari kode atau uraian (mis. M.89, ready mix, mandor)"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {CATS.map(([k, label]) => (
              <button
                key={k}
                className="chip"
                aria-pressed={cat === k}
                onClick={() => setCat(k)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="tw" style={{ maxHeight: 500, overflowY: 'auto' }}>
          <table className="tbl">
            <thead>
              <tr>
                <th style={{ width: 90 }}>Kode</th>
                <th>Uraian</th>
                <th style={{ width: 90 }}>Satuan</th>
                <th className="n" style={{ width: 130 }}>
                  Harga Awal
                </th>
                <th className="n" style={{ width: 150 }}>
                  Harga Revisi
                </th>
                <th className="n" style={{ width: 90 }}>
                  Selisih
                </th>
              </tr>
            </thead>
            <tbody>
              {filtered.length === 0 ? (
                <tr>
                  <td colSpan={6}>
                    <div className="empty">Tidak ada yang cocok.</div>
                  </td>
                </tr>
              ) : (
                filtered.map((x) => {
                  const d = pctDiff(x.orig, x.cur);
                  return (
                    <tr key={x.r}>
                      <td>
                        <span className="cat-code">{x.k}</span>
                      </td>
                      <td>
                        {x.usr ? (
                          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                            <input
                              type="text"
                              style={{ flex: 1 }}
                              defaultValue={x.n}
                              onBlur={(e) =>
                                api
                                  .updateUserHd(x.r, e.target.value, undefined)
                                  .then(onSnapshot)
                              }
                            />
                            <span className="tag soft">BARU</span>
                            <button
                              className="icon-btn danger"
                              onClick={() => handleDeleteUserHd(x.r)}
                            >
                              {armDel === x.r ? 'Yakin?' : 'Hapus'}
                            </button>
                          </div>
                        ) : (
                          <div>{x.n}</div>
                        )}
                        {x.c && <div className="cat-code">{x.c}</div>}
                      </td>
                      <td>
                        {x.usr ? (
                          <input
                            type="text"
                            style={{ width: 72 }}
                            defaultValue={x.u}
                            onBlur={(e) =>
                              api
                                .updateUserHd(x.r, undefined, e.target.value)
                                .then(onSnapshot)
                            }
                          />
                        ) : (
                          x.u
                        )}
                      </td>
                      <td className="num">{fmt(x.orig)}</td>
                      <td className="n">
                        {x.f ? (
                          <div>
                            <span className="num">{fmt(x.cur)}</span>
                            <span className="tag">RUMUS</span>
                          </div>
                        ) : (
                          <input
                            type="number"
                            step="any"
                            style={{
                              width: 135,
                              textAlign: 'right',
                              fontFamily: 'var(--mono)',
                              borderColor: x.is_chg ? 'var(--warn)' : undefined,
                              background: x.is_chg ? 'var(--warn-bg)' : undefined,
                            }}
                            defaultValue={x.cur}
                            key={`${x.r}-${x.cur}`}
                            onBlur={(e) => {
                              const val =
                                e.target.value === ''
                                  ? null
                                  : parseFloat(e.target.value);
                              api
                                .setHdOverride(
                                  x.r,
                                  val !== null && !isNaN(val) ? val : null
                                )
                                .then(onSnapshot);
                            }}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') e.currentTarget.blur();
                            }}
                          />
                        )}
                      </td>
                      <td className="n">
                        {d && (
                          <span className={`pct ${d.pct > 0 ? 'up' : 'dn'}`}>
                            {d.text}
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </section>

      <section className="card">
        <div className="card-hd">
          <h2>Harga Satuan Terdampak ({view.impacted.length})</h2>
        </div>
        <div className="tw">
          <table className="tbl">
            <thead>
              <tr>
                <th>Kode</th>
                <th>Uraian</th>
                <th>Sat</th>
                <th className="n">Sebelum (Rp)</th>
                <th className="n">Sesudah (Rp)</th>
                <th className="n">Selisih</th>
              </tr>
            </thead>
            <tbody>
              {view.impacted.length === 0 ? (
                <tr>
                  <td colSpan={6}>
                    <div className="empty">Belum ada harga satuan yang berubah.</div>
                  </td>
                </tr>
              ) : (
                view.impacted.map((it) => {
                  const d = pctDiff(it.before, it.after);
                  return (
                    <tr key={it.k}>
                      <td>
                        <span className="cat-code">{it.k}</span>
                      </td>
                      <td>{it.u}</td>
                      <td>{it.s}</td>
                      <td className="num">{rp(it.before)}</td>
                      <td className="num">{rp(it.after)}</td>
                      <td className="n">
                        {d && (
                          <span className={`pct ${d.pct > 0 ? 'up' : 'dn'}`}>
                            {d.text}
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}