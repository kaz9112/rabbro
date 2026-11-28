import { useEffect, useState } from 'react';
import { api, rp } from '../../api/commands';
import type {
  AppSnapshot,
  HdView,
  SaveUserAhsPayload,
  UserAhsListItem,
} from '../../types/app';

interface Props {
  snapshot: AppSnapshot;
  deriveTargetCode: string | null;
  onClearDeriveTarget: () => void;
  onSnapshot: (snap: AppSnapshot) => void;
  onOpenInEditor: (code: string) => void;
  onToast: (msg: string) => void;
}

export function UserAhsView({
  snapshot,
  deriveTargetCode,
  onClearDeriveTarget,
  onSnapshot,
  onOpenInEditor,
  onToast,
}: Props) {
  const [list, setList] = useState<UserAhsListItem[]>([]);
  const [hdRows, setHdRows] = useState<HdView['rows']>([]);
  const [armDel, setArmDel] = useState<number | null>(null);

  const [mode, setMode] = useState<'new' | 'der' | 'edit' | 'info' | null>(null);
  const [editId, setEditId] = useState<number | undefined>();
  const [srcK, setSrcK] = useState('');
  const [k, setK] = useState('');
  const [u, setU] = useState('');
  const [s, setS] = useState('');
  const [dv, setDv] = useState('');
  const [newDvCode, setNewDvCode] = useState('');
  const [newDvName, setNewDvName] = useState('');
  const [g, setG] = useState('');
  const [note, setNote] = useState('');

  const [out, setOut] = useState('8');
  const [crew, setCrew] = useState<{ t: string; n: number }[]>([{ t: '', n: 1 }]);
  const [mats, setMats] = useState<{ t: string; u: string; c: number; p: number }[]>([
    { t: '', u: '', c: 1, p: 0 },
  ]);
  const [eqs, setEqs] = useState<{ t: string; u: string; c: number; p: number }[]>([]);

  const ahsCatalog = snapshot.catalog.filter((c) => !c.m);

  const refresh = () => {
    api.getUserAhsList().then(setList).catch((e) => onToast(String(e)));
    api.getHdView().then((v) => setHdRows(v.rows)).catch(() => {});
  };

  useEffect(() => {
    refresh();
  }, [snapshot]);

  const applyDeriveSource = (code: string) => {
    const src = ahsCatalog.find((c) => c.k === code) ?? ahsCatalog[0];
    if (!src) return;
    setSrcK(src.k);
    const sep = src.usr !== null ? '.' : '-R';
    let n = 1;
    while (snapshot.catalog.some((c) => c.k === `${src.k}${sep}${n}`)) n++;
    setK(`${src.k}${sep}${n}`);
    setU(`${src.u} (modifikasi)`);
    setS(src.s);
    setDv(src.dv);
    setG(src.g);
  };

  useEffect(() => {
    if (deriveTargetCode) {
      setMode('der');
      setNote('');
      applyDeriveSource(deriveTargetCode);
      onClearDeriveTarget();
    }
  }, [deriveTargetCode]);

  const openNew = () => {
    setMode('new');
    setEditId(undefined);
    setK('');
    setU('');
    setS('');
    setDv('');
    setNewDvCode('');
    setNewDvName('');
    setG('');
    setOut('8');
    setCrew([{ t: '', n: 1 }]);
    setMats([{ t: '', u: '', c: 1, p: 0 }]);
    setEqs([]);
  };

  const openDerive = () => {
    setMode('der');
    setEditId(undefined);
    setNote('');
    if (ahsCatalog[0]) applyDeriveSource(ahsCatalog[0].k);
  };

  const openEdit = (item: UserAhsListItem) => {
    setMode(item.is_spec ? 'edit' : 'info');
    setEditId(item.id);
    setK(item.k);
    setU(item.u);
    setS(item.s);
    setDv(item.dv);
    setG(item.g);
    if (item.spec) {
      setOut(String(item.spec.out));
      setCrew(item.spec.crew.map((x) => ({ ...x })));
      setMats(item.spec.mats.map((x) => ({ ...x })));
      setEqs(item.spec.eqs.map((x) => ({ ...x })));
    }
  };

  const handleSave = async () => {
    if (!mode) return;
    const payload: SaveUserAhsPayload = {
      mode,
      id: editId,
      src_k: mode === 'der' ? srcK : undefined,
      k,
      u,
      s,
      dv,
      new_dv_code: dv === '__new' ? newDvCode : undefined,
      new_dv_name: dv === '__new' ? newDvName : undefined,
      g,
      note: mode === 'der' ? note : undefined,
      spec:
        mode === 'new' || mode === 'edit'
          ? {
              out: parseFloat(out) || 0,
              crew,
              mats,
              eqs,
            }
          : undefined,
    };

    try {
      const snap = await api.saveUserAhs(payload);
      onSnapshot(snap);
      setMode(null);
      onToast(`AHS tersimpan: ${k}`);
    } catch (e) {
      onToast(String(e));
    }
  };

  const handleDelete = async (id: number) => {
    if (armDel !== id) {
      setArmDel(id);
      setTimeout(() => setArmDel((c) => (c === id ? null : c)), 3000);
      return;
    }
    try {
      setArmDel(null);
      onSnapshot(await api.deleteUserAhs(id));
      onToast('AHS dihapus');
    } catch (e) {
      onToast(String(e));
    }
  };

  const findHd = (txt: string, cat: string) => {
    const t = txt.trim().toLowerCase();
    return hdRows.find(
      (h) =>
        h.k.startsWith(cat) &&
        (`${h.k} | ${h.n}`.toLowerCase() === t || h.k.toLowerCase() === t)
    );
  };

  return (
    <section className="card">
      <div className="card-hd">
        <h2>AHS Buatan Sendiri</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <button onClick={openDerive}>Turunkan dari AHS yang Ada</button>
          <button className="pri" onClick={openNew}>
            + Buat AHS dari Nol
          </button>
        </div>
      </div>

      {mode && (
        <div className="subpanel">
          <h3 style={{ marginTop: 0, fontSize: 14 }}>
            {mode === 'new' && 'Buat AHS dari Nol'}
            {mode === 'der' && 'Turunkan dari AHS yang Ada'}
            {mode === 'edit' && `Ubah Susunan AHS ${k}`}
            {mode === 'info' && `Ubah Info AHS ${k}`}
          </h3>

          <div className="form-grid">
            {mode === 'der' && (
              <label className="field" style={{ gridColumn: '1 / -1' }}>
                AHS Asal
                <select
                  value={srcK}
                  onChange={(e) => applyDeriveSource(e.target.value)}
                >
                  {ahsCatalog.map((it) => (
                    <option key={it.k} value={it.k}>
                      {it.k} — {it.u.slice(0, 80)}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <label className="field">
              Kode AHS
              <input
                type="text"
                readOnly={mode !== 'new'}
                value={k}
                placeholder="mis. 03.A.001"
                onChange={(e) => setK(e.target.value)}
              />
            </label>
            <label className="field">
              Satuan
              <input
                type="text"
                value={s}
                placeholder="m3, m2, kg"
                onChange={(e) => setS(e.target.value)}
              />
            </label>
            <label className="field" style={{ gridColumn: '1 / -1' }}>
              Uraian Pekerjaan
              <input
                type="text"
                value={u}
                onChange={(e) => setU(e.target.value)}
              />
            </label>
            <label className="field">
              Divisi
              <select value={dv} onChange={(e) => setDv(e.target.value)}>
                <option value="">Pilih divisi...</option>
                {Object.entries(snapshot.divs)
                  .sort(([a], [b]) => a.localeCompare(b))
                  .map(([code, name]) => (
                    <option key={code} value={code}>
                      Divisi {code} {name}
                    </option>
                  ))}
                <option value="__new">+ Divisi baru...</option>
              </select>
            </label>
            {dv === '__new' && (
              <>
                <label className="field">
                  Kode Divisi Baru
                  <input
                    type="text"
                    value={newDvCode}
                    placeholder="mis. 99"
                    onChange={(e) => setNewDvCode(e.target.value)}
                  />
                </label>
                <label className="field">
                  Nama Divisi Baru
                  <input
                    type="text"
                    value={newDvName}
                    onChange={(e) => setNewDvName(e.target.value)}
                  />
                </label>
              </>
            )}
            <label className="field">
              Subdivisi
              <input
                type="text"
                value={g}
                placeholder="mis. 03 30 00 Cast-in-Place Concrete"
                onChange={(e) => setG(e.target.value)}
              />
            </label>
            {mode === 'der' && (
              <label className="field" style={{ gridColumn: '1 / -1' }}>
                Catatan Perubahan (opsional)
                <input
                  type="text"
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                />
              </label>
            )}
          </div>

          {(mode === 'new' || mode === 'edit') && (
            <>
              <h4 style={{ margin: '12px 0 6px' }}>1. Crew (Regu Kerja)</h4>
              <table className="tbl" style={{ marginBottom: 8 }}>
                <thead>
                  <tr>
                    <th>Upah (dari Harga Dasar)</th>
                    <th style={{ width: 130 }}>Jumlah Orang</th>
                    <th style={{ width: 42 }}></th>
                  </tr>
                </thead>
                <tbody>
                  {crew.map((row, idx) => {
                    const hd = findHd(row.t, 'L');
                    return (
                      <tr key={idx}>
                        <td>
                          <input
                            type="text"
                            list="ua-dl-L"
                            style={{ width: '100%' }}
                            value={row.t}
                            placeholder="Cari upah, mis. L.01"
                            onChange={(e) => {
                              const next = [...crew];
                              next[idx] = { ...row, t: e.target.value };
                              setCrew(next);
                            }}
                          />
                          {hd && (
                            <div className="cat-code">
                              Rp {rp(hd.cur)} / {hd.u}
                            </div>
                          )}
                        </td>
                        <td>
                          <input
                            type="number"
                            min={0}
                            step="any"
                            style={{ width: '100%' }}
                            value={row.n}
                            onChange={(e) => {
                              const next = [...crew];
                              next[idx] = {
                                ...row,
                                n: parseFloat(e.target.value) || 0,
                              };
                              setCrew(next);
                            }}
                          />
                        </td>
                        <td>
                          <button
                            className="icon-btn danger"
                            onClick={() =>
                              setCrew(crew.filter((_, i) => i !== idx))
                            }
                          >
                            ✕
                          </button>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              <button onClick={() => setCrew([...crew, { t: '', n: 1 }])}>
                + Anggota Crew
              </button>

              <h4 style={{ margin: '14px 0 6px' }}>2. Produktivitas</h4>
              <div className="form-grid">
                <label className="field">
                  Daily Output ({s || 'satuan'} / hari)
                  <input
                    type="number"
                    min={0}
                    step="any"
                    value={out}
                    onChange={(e) => setOut(e.target.value)}
                  />
                </label>
              </div>

              {(
                [
                  ['3. Material / Bahan', 'M', mats, setMats],
                  ['4. Peralatan', 'E', eqs, setEqs],
                ] as const
              ).map(([title, cat, arr, setter]) => (
                <div key={cat} style={{ marginTop: 14 }}>
                  <h4 style={{ margin: '0 0 6px' }}>{title}</h4>
                  <table className="tbl" style={{ marginBottom: 8 }}>
                    <thead>
                      <tr>
                        <th>Item (Harga Dasar atau ketik manual)</th>
                        <th style={{ width: 90 }}>Satuan</th>
                        <th style={{ width: 110 }}>Koefisien</th>
                        <th style={{ width: 140 }}>Harga (Rp)</th>
                        <th style={{ width: 42 }}></th>
                      </tr>
                    </thead>
                    <tbody>
                      {arr.map((row, idx) => {
                        const hd = findHd(row.t, cat);
                        return (
                          <tr key={idx}>
                            <td>
                              <input
                                type="text"
                                list={`ua-dl-${cat}`}
                                style={{ width: '100%' }}
                                value={row.t}
                                onChange={(e) => {
                                  const next = [...arr];
                                  const matched = findHd(e.target.value, cat);
                                  next[idx] = {
                                    ...row,
                                    t: e.target.value,
                                    u: matched && !row.u ? matched.u : row.u,
                                  };
                                  setter(next);
                                }}
                              />
                            </td>
                            <td>
                              <input
                                type="text"
                                style={{ width: '100%' }}
                                value={row.u}
                                placeholder={hd?.u || 'sat'}
                                onChange={(e) => {
                                  const next = [...arr];
                                  next[idx] = { ...row, u: e.target.value };
                                  setter(next);
                                }}
                              />
                            </td>
                            <td>
                              <input
                                type="number"
                                min={0}
                                step="any"
                                style={{ width: '100%' }}
                                value={row.c}
                                onChange={(e) => {
                                  const next = [...arr];
                                  next[idx] = {
                                    ...row,
                                    c: parseFloat(e.target.value) || 0,
                                  };
                                  setter(next);
                                }}
                              />
                            </td>
                            <td>
                              {hd ? (
                                <span className="num">Rp {rp(hd.cur)}</span>
                              ) : (
                                <input
                                  type="number"
                                  min={0}
                                  step="any"
                                  style={{ width: '100%' }}
                                  value={row.p}
                                  onChange={(e) => {
                                    const next = [...arr];
                                    next[idx] = {
                                      ...row,
                                      p: parseFloat(e.target.value) || 0,
                                    };
                                    setter(next);
                                  }}
                                />
                              )}
                            </td>
                            <td>
                              <button
                                className="icon-btn danger"
                                onClick={() =>
                                  setter(arr.filter((_, i) => i !== idx))
                                }
                              >
                                ✕
                              </button>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                  <button
                    onClick={() =>
                      setter([...arr, { t: '', u: '', c: 1, p: 0 }])
                    }
                  >
                    + Tambah {cat === 'M' ? 'Material' : 'Alat'}
                  </button>
                </div>
              ))}
            </>
          )}

          <div style={{ display: 'flex', gap: 8, marginTop: 16 }}>
            <button className="pri" onClick={handleSave}>
              Simpan AHS
            </button>
            <button onClick={() => setMode(null)}>Batal</button>
          </div>
        </div>
      )}

      <div className="tw">
        <table className="tbl">
          <thead>
            <tr>
              <th>Kode</th>
              <th>Uraian</th>
              <th>Divisi &amp; Subdivisi</th>
              <th>Sat</th>
              <th className="n">Harga Satuan (Rp)</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {list.length === 0 ? (
              <tr>
                <td colSpan={6}>
                  <div className="empty">
                    Belum ada AHS buatan sendiri. Klik "+ Buat AHS dari Nol" atau
                    "Turunkan dari AHS yang Ada".
                  </div>
                </td>
              </tr>
            ) : (
              list.map((it) => (
                <tr key={it.id}>
                  <td>
                    <span className="cat-code">{it.k}</span>
                  </td>
                  <td>
                    <div>{it.u}</div>
                    <div className="cat-code">
                      {it.from ? `Turunan dari ${it.from}` : 'Dibuat dari nol'}
                    </div>
                  </td>
                  <td>
                    <div>Divisi {it.dv}</div>
                    <div className="cat-code">{it.g}</div>
                  </td>
                  <td>{it.s}</td>
                  <td className="num">{rp(it.price)}</td>
                  <td style={{ whiteSpace: 'nowrap', textAlign: 'right' }}>
                    <button
                      className="ghost"
                      onClick={() => onOpenInEditor(it.k)}
                    >
                      Edit Kalkulasi
                    </button>{' '}
                    <button className="ghost" onClick={() => openEdit(it)}>
                      {it.is_spec ? 'Ubah Susunan' : 'Ubah Info'}
                    </button>{' '}
                    <button
                      className="ghost"
                      onClick={() => {
                        setMode('der');
                        setNote('');
                        applyDeriveSource(it.k);
                      }}
                    >
                      Turunkan
                    </button>{' '}
                    <button
                      className="icon-btn danger"
                      onClick={() => handleDelete(it.id)}
                    >
                      {armDel === it.id
                        ? 'Yakin?'
                        : `Hapus${it.used_count ? ` (${it.used_count})` : ''}`}
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      <div hidden>
        {(['L', 'M', 'E'] as const).map((cat) => (
          <datalist key={cat} id={`ua-dl-${cat}`}>
            {hdRows
              .filter((h) => h.k.startsWith(cat))
              .map((h) => (
                <option key={h.r} value={`${h.k} | ${h.n}`} />
              ))}
          </datalist>
        ))}
      </div>
    </section>
  );
}