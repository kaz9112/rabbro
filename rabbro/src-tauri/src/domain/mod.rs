// Domain module
pub mod ahs_builder;
pub mod boq;
pub mod validation;

#[cfg(test)]
mod tests {
    use super::ahs_builder::{build_all_ahs_items, derive_user_ahs, gen_spec_user_ahs};
    use super::boq::{build_all_catalog, build_boq_summary, effective_divs, norm_g};
    use crate::engine::recalc::{build_effective_cells, effective_names};
    use crate::models::catalog::StaticDatabase;
    use crate::models::state::{BoqLine, ComponentSpec, CrewSpec, ProjectState, UserAhsSpec};

    #[test]
    fn norm_g_pads_short_codes() {
        assert_eq!(norm_g("03 30 Cast"), "03 30 00 Cast");
        assert_eq!(norm_g("03 30 00 Cast"), "03 30 00 Cast");
    }

    #[test]
    fn derived_ahs_matches_source_price() {
        let db = StaticDatabase::load_embedded().unwrap();
        let mut state = ProjectState::default();
        let cells = build_effective_cells(&db, &state);
        let names = effective_names(&db);
        let src = db.payload.items.first().unwrap();

        let derived = derive_user_ahs(
            0,
            src,
            format!("{}-R1", src.k),
            src.u.clone(),
            src.s.clone(),
            src.dv.clone(),
            src.g.clone(),
            "test",
            &cells,
            &names,
        )
        .unwrap();
        state.uahs.push(derived);

        let all = build_all_ahs_items(&db, &state).unwrap();
        let d_item = all.iter().find(|i| i.k == format!("{}-R1", src.k)).unwrap();
        assert!((d_item.h - src.h0).abs() < 1e-4);
        assert!((d_item.b - src.b0).abs() < 1e-4);
    }

    #[test]
    fn custom_spec_ahs_and_boq_grouping_work() {
        let db = StaticDatabase::load_embedded().unwrap();
        let mut state = ProjectState::default();
        let spec = UserAhsSpec {
            crew: vec![CrewSpec { t: "L.01".into(), n: 2.0 }],
            out: 10.0,
            mats: vec![ComponentSpec { t: "M.01".into(), u: "bh".into(), c: 8.0, p: 0.0 }],
            eqs: vec![],
        };
        let ua = gen_spec_user_ahs(
            0,
            "04.USR.01".into(),
            "Dinding custom".into(),
            "m2".into(),
            "04".into(),
            "04 22 00 Concrete Unit Masonry".into(),
            &spec,
            &db.payload.hd,
            None,
        )
        .unwrap();
        state.uahs.push(ua);
        state.lines.push(BoqLine {
            id: 1,
            k: "04.USR.01".into(),
            u: "Dinding custom".into(),
            s: "m2".into(),
            qty: 10.0,
            note: String::new(),
            custom: false,
            man: false,
            price: 0.0,
            dv: String::new(),
            g: String::new(),
        });

        let ahs = build_all_ahs_items(&db, &state).unwrap();
        let cat = build_all_catalog(&db, &ahs);
        let divs = effective_divs(&db, &state);
        let summary = build_boq_summary(&state, &ahs, &cat, &divs);
        assert!(summary.subtotal > 0.0);
        assert_eq!(summary.item_count, 1);
    }
}