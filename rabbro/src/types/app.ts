export interface BoqLine {
    id: number;
    k: string;
    u: string;
    s: string;
    qty: number;
    note: string;
    custom: boolean;
    man: boolean;
    price: number;
    dv: string;
    g: string;
  }
  
  export interface UserHdItem {
    id: number;
    r: number;
    k: string;
    n: string;
    u: string;
    c: string;
    v: number;
  }
  
  export interface UserAhsCrewSpec {
    t: string;
    n: number;
  }
  
  export interface UserAhsCompSpec {
    t: string;
    u: string;
    c: number;
    p: number;
  }
  
  export interface UserAhsSpec {
    crew: UserAhsCrewSpec[];
    out: number;
    mats: UserAhsCompSpec[];
    eqs: UserAhsCompSpec[];
  }
  
  export interface ProjectState {
    fname: string;
    title: string;
    sub: string;
    proj: string;
    company: string;
    by: string;
    date: string;
    rev: string;
    basis: 'h' | 'b';
    ppn: number;
    lines: BoqLine[];
    hd: Record<string, number>;
    ov: Record<string, string | number>;
    uahs: unknown[];
    udivs: Record<string, string>;
    unext: number;
    uhd: UserHdItem[];
    uhnext: number;
    theme: string;
    font: string;
  }
  
  export interface CatalogEntry {
    k: string;
    u: string;
    s: string;
    g: string;
    dv: string;
    m: boolean;
    usr: number | null;
    price: number;
    price0: number;
  }
  
  export interface BoqRenderLine {
    line: BoqLine;
    rate: number;
    amount: number;
  }
  
  export interface BoqSubgroup {
    code: string;
    name: string;
    full_g: string;
    subtotal: number;
    lines: BoqRenderLine[];
  }
  
  export interface BoqDivisionGroup {
    key: string;
    label: string;
    subtotal: number;
    subgroups: BoqSubgroup[];
    direct_lines: BoqRenderLine[];
  }
  
  export interface BoqSummary {
    groups: BoqDivisionGroup[];
    subtotal: number;
    tax: number;
    grand_total: number;
    line_count: number;
  }
  
  export interface AppSnapshot {
    state: ProjectState;
    divs: Record<string, string>;
    catalog: CatalogEntry[];
    boq: BoqSummary;
    dirty: boolean;
    last_file_time: string | null;
    default_filename: string;
    hd_count: number;
    ov_count: number;
    ua_count: number;
  }
  
  export interface HdRowView {
    r: number;
    k: string;
    n: string;
    u: string;
    c: string;
    f: boolean;
    usr: boolean;
    orig: number;
    cur: number;
    is_chg: boolean;
  }
  
  export interface ImpactedAhsView {
    k: string;
    u: string;
    s: string;
    before: number;
    after: number;
  }
  
  export interface HdView {
    rows: HdRowView[];
    impacted: ImpactedAhsView[];
    hd_count: number;
    total_ahs: number;
  }
  
  export interface AhsSelectorItem {
    idx: number;
    k: string;
    u: string;
    dv: string;
    dv_name: string;
  }
  
  export interface AhsGridCell {
    addr: string;
    is_str: boolean;
    is_formula: boolean;
    is_modified: boolean;
    display: string;
    num_val: number | null;
    raw_input: string;
    orig_raw: string;
  }
  
  export type AhsGridRow =
    | { kind: 'banner'; text: string; is_section: boolean }
    | { kind: 'header'; cols: string[] }
    | { kind: 'data'; is_total: boolean; cells: (AhsGridCell | null)[] };
  
  export interface RelinkRowView {
    addr: string;
    label: string;
    current_hd_label: string;
    is_pure_ref: boolean;
    raw_formula: string;
    price: number;
  }
  
  export interface AhsEditorView {
    selector: AhsSelectorItem[];
    selected_idx: number;
    k: string;
    unit: string;
    bare: number;
    bare0: number;
    total: number;
    total0: number;
    hours: number;
    hours0: number;
    rows: AhsGridRow[];
    relink_rows: RelinkRowView[];
    is_user: boolean;
    is_spec: boolean;
    ov_count: number;
  }
  
  export interface UserAhsListItem {
    id: number;
    k: string;
    u: string;
    s: string;
    dv: string;
    g: string;
    from: string;
    is_spec: boolean;
    spec: UserAhsSpec | null;
    price: number;
    used_count: number;
  }
  
  export interface SaveUserAhsPayload {
    mode: 'new' | 'der' | 'edit' | 'info';
    id?: number;
    src_k?: string;
    k: string;
    u: string;
    s: string;
    dv: string;
    new_dv_code?: string;
    new_dv_name?: string;
    g: string;
    note?: string;
    spec?: UserAhsSpec;
  }
  
  export interface OpenProjectResult {
    snapshot: AppSnapshot;
    filename: string;
    skipped: number;
  }