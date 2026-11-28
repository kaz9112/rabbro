import { useEffect, useState } from 'react';
import { api, f4, pctDiff, rp } from '../../api/commands';
import type { AhsEditorView as AhsEditorData, AppSnapshot, HdView } from '../../types/app';

interface Props {
  snapshot: AppSnapshot;
  selectedIdx: number;
  onSelectIdx: (idx: number) => void;
  onSnapshot: (snap: AppSnapshot) => void;
  onToast: (msg: string) => void;
  onDeriveAhs: (srcCode: string) => void;
}

export function AhsEditorView({
  snapshot,
  selectedIdx,
  onSelectIdx,
  onSnapshot,
  onToast,
  onDeriveAhs,
}: Props) {
  const [data, setData] = useState<AhsEditorData | null>(null);
  const [hdList, setHdList] = useState<HdView['rows']>([]);
  const [focusAddr, setFocusAddr] = useState<string | null>(null);
  const [draftVal, setDraftVal] = useState('');
  const [armResetAll, setArmResetAll] = useState(false);

  const load = () => {
    api
      .getAhsEditorView(selectedIdx)
      .then(setData)
      .catch((e) => onToast(String(e)));
  };

  useEffect(() => {
    load();
    api.getHdView().then((v) => setHdList(v.rows)).catch(() => {});
  }, [selectedIdx, snapshot]);

  const handleCommit = async (addr: string, rawInput: string) => {
    setFocusAddr(null);
    if (!data) return;
    try {
      const snap = await api.commitAhsCell(data.selected_idx, addr, draftVal);
      onSnapshot(snap);
    } catch (e) {
      onToast(String(e));
      setDraftVal(rawInput);
    }
  };

  const handleResetAll = async () => {
    if (!data?.ov_count) return;
    if (!armResetAll) {
      setArmResetAll(true);
      setTimeout(() => setArmResetAll(false), 3000);
      return;
    }
    setArmResetAll(false);
    onSnapshot(await api.resetAhsOverrides(null));
    onToast('Semua perubahan AHS dikembalikan ke awal');
  };

  if (!data) return <div className="card">Memuat Kalkulasi AHS...</div>;

  // Group selector options by division
  const groupedSel: { dv: string; label: string; items: typeof data.selector }[] = [];
  for (const item of data.selector) {
    let grp = groupedSel.find((g) => g.dv === item.dv);
    if (!grp) {
      grp = { dv: item.dv, label: `Divisi ${item.dv} ${item.dv_name}`, items: [] };
      groupedSel.push(grp);
    }
    grp.items.push(item);
  }

  const bareDiff = pctDiff(data.bare0, data.bare);
  const totDiff = pctDiff(data.total0, data.total);
  const hrsDiff = pctDiff(data.hours0, data.hours);

  return (
    <section className="card">
      <div className="card-hd">
        <h2>Kalkulasi Analisa Harga Satuan</h2>
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
          <button onClick={() => onDeriveAhs(data.k)}>Turunkan AHS Ini</button>
          <button
            onClick={() =>
              api.resetAhsOverrides(data.selected_idx).then(onSnapshot)
            }
          >
            Reset AHS Ini
          </button>
          <button onClick={handleResetAll}>
            {armResetAll ? 'Klik lagi untuk reset semua' : 'Reset Semua AHS'}
          </button>
        </div>
      </div>

      <div className="filter-bar">
        <select
          style={{ flex: 1, minWidth: 280 }}
          value={data.selected_idx}
          onChange={(e) => onSelectIdx(Number(e.target.value))}
          aria-label="Pilih AHS"
        >
          {groupedSel.map((g) => (
            <optgroup key={g.dv} label={g.label}>
              {g.items.map((it) => (
                <option key={it.idx} value={it.idx}>
                  {it.k} — {it.u.slice(0, 75)}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </div>

      <div className="kpi-grid">
        <div className="kpi-card">
          <div className="l">Bare Cost</div>
          <div className="v">Rp {rp(data.bare)}</div>
          <div className="d">
            {bareDiff ? `awal Rp ${rp(data.bare0)} (${bareDiff.text})` : 'sama dengan awal'}
          </div>
        </div>
        <div className="kpi-card">
          <div className="l">Total incl. O&amp;P (Dibulatkan)</div>
          <div className="v">Rp {rp(data.total)}</div>
          <div className="d">
            {totDiff ? `awal Rp ${rp(data.total0)} (${totDiff.text})` : 'sama dengan awal'}
          </div>
        </div>
        <div className="kpi-card">
          <div className="l">Jam-orang per {data.unit}</div>
          <div className="v">{f4(data.hours)}</div>
          <div className="d">
            {hrsDiff ? `awal ${f4(data.hours0)} (${hrsDiff.text})` : 'sama dengan awal'}
          </div>
        </div>
      </div>

      <div className="tw">
        <table className="ahs-grid">
          <colgroup>
            <col style={{ width: '15%' }} />
            <col style={{ width: '41%' }} />
            <col style={{ width: '11%' }} />
            <col style={{ width: '11%' }} />
            <col style={{ width: '11%' }} />
            <col style={{ width: '11%' }} />
          </colgroup>
          <tbody>
            {data.rows.map((row, rIdx) => {
              if (row.kind === 'banner') {
                return (
                  <tr key={rIdx} className={row.is_section ? 'sec' : 'info'}>
                    <td colSpan={6}>{row.text}</td>
                  </tr>
                );
              }
              if (row.kind === 'header') {
                return (
                  <tr key={rIdx} className="hdr">
                    {row.cols.map((c, ci) => (
                      <td key={ci} className={ci >= 2 ? 'r' : ''}>
                        {c}
                      </td>
                    ))}
                  </tr>
                );
              }
              return (
                <tr key={rIdx} className={row.is_total ? 'sm' : ''}>
                  {row.cells.map((cell, ci) => {
                    if (!cell) return <td key={ci}></td>;
                    if (cell.is_str) return <td key={ci}>{cell.display}</td>;

                    const isFocused = focusAddr === cell.addr;
                    const displayStr =
                      cell.num_val !== null ? f4(cell.num_val) : cell.display;

                    return (
                      <td key={ci}>
                        <input
                          type="text"
                          className={
                            cell.is_modified
                              ? 'chg'
                              : cell.is_formula
                                ? 'fx'
                                : ''
                          }
                          title={`Asli: ${cell.orig_raw}`}
                          value={isFocused ? draftVal : displayStr}
                          onFocus={(e) => {
                            setFocusAddr(cell.addr);
                            setDraftVal(cell.raw_input);
                            setTimeout(() => e.target.select(), 0);
                          }}
                          onChange={(e) => setDraftVal(e.target.value)}
                          onBlur={() => handleCommit(cell.addr, cell.raw_input)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              e.preventDefault();
                              e.currentTarget.blur();
                            } else if (e.key === 'Escape') {
                              setDraftVal(cell.raw_input);
                              setFocusAddr(null);
                              e.currentTarget.blur();
                            }
                          }}
                        />
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {data.is_user && data.is_spec && (
        <p className="note">
          AHS ini dibuat dari nol. Untuk mengganti komponen ke item Harga Dasar
          lain, buka tab <b>AHS Buatan Sendiri</b> dan klik <b>Ubah susunan</b>.
        </p>
      )}

      {data.relink_rows.length > 0 && (
        <div style={{ marginTop: 20 }}>
          <h3 style={{ fontSize: 13.5, marginBottom: 8 }}>Tautan ke Harga Dasar</h3>
          <div className="tw">
            <table className="tbl">
              <thead>
                <tr>
                  <th>Baris</th>
                  <th>Item Harga Dasar (ganti dengan item lain)</th>
                  <th className="n">Harga per satuan AHS</th>
                </tr>
              </thead>
              <tbody>
                {data.relink_rows.map((rl) => (
                  <tr key={rl.addr}>
                    <td>{rl.label}</td>
                    <td>
                      <input
                        type="text"
                        list="ahs-dl-all"
                        style={{ width: '100%' }}
                        defaultValue={rl.current_hd_label}
                        onChange={(e) => {
                          const val = e.target.value;
                          const matched = hdList.some(
                            (h) => `${h.k} | ${h.n}` === val
                          );
                          if (matched) {
                            api
                              .relinkHdInAhs(data.selected_idx, rl.addr, val)
                              .then((snap) => {
                                onSnapshot(snap);
                                onToast('Tautan Harga Dasar diperbarui');
                              })
                              .catch((err) => onToast(String(err)));
                          }
                        }}
                      />
                      {!rl.is_pure_ref && (
                        <div className="note" style={{ marginTop: 3 }}>
                          Rumus asli: {rl.raw_formula}
                        </div>
                      )}
                    </td>
                    <td className="num">{rp(rl.price)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <datalist id="ahs-dl-all">
            {hdList
              .filter((h) => ['L', 'M', 'E'].includes(h.k[0]))
              .map((h) => (
                <option key={h.r} value={`${h.k} | ${h.n}`} />
              ))}
          </datalist>
        </div>
      )}

      <p className="note">
        Klik angka mana saja untuk mengubah nilai atau rumus (diawali <code>=</code>,
        gunakan <code>HD!D7</code> untuk Harga Dasar). Kosongkan isi untuk
        mengembalikan ke nilai awal.
      </p>
    </section>
  );
}