use crate::gui::bulldozer::BulldozerState;
use crate::gui::lotbrush::LotBrushResource;
use crate::gui::roadeditor::RoadEditorResource;
use crate::gui::specialbuilding::{SpecialBuildKind, SpecialBuildingResource};
use crate::gui::windows::settings::Settings;
use crate::gui::windows::ImguiWindows;
use crate::gui::{InspectedEntity, RoadBuildResource, Tool, UiTex, UiTextures};
use crate::input::{KeyCode, KeyboardInfo};
use crate::inputmap::{InputAction, InputMap};
use crate::uiworld::UiWorld;
use common::saveload::Encoder;
use egregoria::economy::{Government, ItemRegistry};
use egregoria::map::{
    BuildingGen, BuildingKind, LanePatternBuilder, LightPolicy, LotKind, StraightRoadGen,
    TurnPolicy,
};
use egregoria::souls::goods_company::GoodsCompanyRegistry;
use egregoria::utils::time::GameTime;
use egregoria::Egregoria;
use egui::{Context, RichText, Ui, Widget, Window};
use egui_inspect::{
    InspectArgsDefault, InspectArgsStruct, InspectRenderDefault, InspectRenderStruct,
};
use geom::Vec2;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Gui {
    pub(crate) windows: ImguiWindows,
    #[serde(skip)]
    pub(crate) last_save: Instant,
    #[serde(skip)]
    pub(crate) last_gui_save: Instant,
    #[serde(skip)]
    pub(crate) n_cars: i32,
    #[serde(skip)]
    pub(crate) n_pedestrians: i32,
    pub(crate) depause_warp: u32,
    #[serde(skip)]
    pub(crate) hidden: bool,
}

impl Default for Gui {
    fn default() -> Self {
        Self {
            windows: ImguiWindows::default(),
            last_save: Instant::now(),
            last_gui_save: Instant::now(),
            n_cars: 100,
            n_pedestrians: 100,
            depause_warp: 1,
            hidden: false,
        }
    }
}

impl Gui {
    pub(crate) fn render(&mut self, ui: &Context, uiworld: &mut UiWorld, goria: &Egregoria) {
        //let _tw = ui.push_style_color(StyleColor::WindowBg, common::config().gui_bg_col.into());
        //let _tt = ui.push_style_color(StyleColor::TitleBg, common::config().gui_title_col.into());
        self.menu_bar(ui, uiworld, goria);

        Self::inspector(ui, uiworld, goria);

        self.windows.render(ui, uiworld, goria);

        Self::toolbox(ui, uiworld, goria);

        self.time_controls(ui, uiworld, goria);

        self.auto_save(uiworld);
    }

    pub(crate) fn auto_save(&mut self, uiworld: &mut UiWorld) {
        let every = uiworld.read::<Settings>().auto_save_every.into();
        if let Some(every) = every {
            if self.last_save.elapsed() > every {
                uiworld.please_save = true;
                uiworld.save_to_disk();
                self.last_save = Instant::now();
            }
        }

        if self.last_gui_save.elapsed() > Duration::from_secs(1) {
            common::saveload::JSON::save_silent(self, "gui");
            self.last_gui_save = Instant::now();
        }
    }

    pub(crate) fn toolbox(ui: &Context, uiworld: &mut UiWorld, goria: &Egregoria) {
        #[derive(Copy, Clone)]
        pub(crate) enum Tab {
            Hand,
            Roadbuild,
            Roadcurved,
            Roadeditor,
            Lotbrush,
            Roadbuilding,
            Bulldozer,
            Train,
        }
        uiworld.check_present(|| Tab::Hand);

        if uiworld
            .read::<InputMap>()
            .just_act
            .contains(&InputAction::Close)
        {
            *uiworld.write::<Tool>() = Tool::Hand;
            *uiworld.write::<Tab>() = Tab::Hand;
        }

        let [w, h] = ui.available_rect().size().into();
        //        let _tok1 = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
        //        let _tok2 = ui.push_style_var(StyleVar::WindowBorderSize(0.0));
        //        let _tok3 = ui.push_style_var(StyleVar::WindowRounding(0.0));
        //        let _tok4 = ui.push_style_var(StyleVar::ItemSpacing([0.0, 0.0]));

        let toolbox_w = 80.0;

        let tools = [
            (UiTex::Road, Tab::Roadbuild, Tool::RoadbuildStraight),
            (UiTex::Curved, Tab::Roadcurved, Tool::RoadbuildCurved),
            (UiTex::RoadEdit, Tab::Roadeditor, Tool::RoadEditor),
            (UiTex::LotBrush, Tab::Lotbrush, Tool::LotBrush),
            (UiTex::Buildings, Tab::Roadbuilding, Tool::SpecialBuilding),
            (UiTex::Bulldozer, Tab::Bulldozer, Tool::Bulldozer),
            (UiTex::AddTrain, Tab::Train, Tool::Train),
        ];

        Window::new("Toolbox")
            .min_width(toolbox_w)
            .fixed_pos([w - toolbox_w, h * 0.5 - 30.0])
            .vscroll(false)
            .title_bar(true)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .show(ui, |ui| {
                let cur_tab = *uiworld.read::<Tab>();

                for (name, tab, default_tool) in &tools {
                    let alpha = if std::mem::discriminant(tab) == std::mem::discriminant(&cur_tab) {
                        1.0
                    } else {
                        0.6
                    };
                    if egui::ImageButton::new(
                        uiworld.read::<UiTextures>().get(*name),
                        [toolbox_w, 30.0],
                    )
                    .tint([1.0, 1.0, 1.0, alpha])
                    .frame(false)
                    .ui(ui)
                    .clicked()
                    {
                        uiworld.insert::<Tool>(*default_tool);
                        uiworld.insert(*tab);
                    }
                }
            });

        if matches!(*uiworld.read::<Tab>(), Tab::Roadeditor) {
            let state = &mut *uiworld.write::<RoadEditorResource>();
            if let Some(ref mut v) = state.inspect {
                let dirty = &mut state.dirty;
                Window::new("Road Properties")
                    .fixed_size([150.0, 200.0])
                    .fixed_pos([w - 150.0 - toolbox_w, h * 0.5 - 30.0])
                    .vscroll(false)
                    .title_bar(true)
                    .collapsible(false)
                    .resizable(false)
                    .show(ui, |ui| {
                        ui.label("Light policy");
                        *dirty |= <LightPolicy as InspectRenderDefault<LightPolicy>>::render_mut(
                            &mut v.light_policy,
                            "",
                            ui,
                            &InspectArgsDefault {
                                header: Some(false),
                                indent_children: Some(false),
                                ..Default::default()
                            },
                        );
                        ui.add_space(10.0);
                        ui.label("Turn policy");
                        *dirty |= <TurnPolicy as InspectRenderDefault<TurnPolicy>>::render_mut(
                            &mut v.turn_policy,
                            "Turn policy",
                            ui,
                            &InspectArgsDefault {
                                header: Some(false),
                                indent_children: Some(false),
                                ..Default::default()
                            },
                        );
                    });
            }
        }

        if matches!(*uiworld.read::<Tab>(), Tab::Train) {
            let rbw = 150.0;
            Window::new("Trains")
                .fixed_size([rbw, 83.0])
                .fixed_pos([w - rbw - toolbox_w, h * 0.5 - 30.0])
                .hscroll(false)
                .title_bar(true)
                .collapsible(false)
                .resizable(false)
                .build(ui, |ui: &mut Ui| {
                    ui.style_mut().spacing.interact_size = [rbw, 30.0].into();

                    let addtrain = RichText::new("Add Train");
                    if *uiworld.read::<Tool>() == Tool::Train {
                        addtrain.strong();
                    };
                    if ui.button(addtrain).clicked() {
                        *uiworld.write::<Tool>() = Tool::Train;
                    }

                    /*
                    if ui.button_with_size("Trainstation", [rbw, 30.0]) {
                        *uiworld.write::<Tool>() = Tool::SpecialBuilding;

                        let h = LanePatternBuilder::new().rail(true).n_lanes(1).width();

                        uiworld.write::<SpecialBuildingResource>().opt = Some(SpecialBuildKind {
                            make: Box::new(move |args, commands| {
                                let d = args.obb.axis()[0].z(0.0) * 0.5;
                                let off = args.obb.axis()[1].z(0.0).normalize_to(h * 0.5 + 10.0);
                                commands.map_build_trainstation(
                                    args.mpos - d - off,
                                    args.mpos + d - off,
                                );
                            }),
                            w: h + 15.0,
                            h: 230.0,
                            asset: "trainstation.glb".to_string(),
                            road_snap: false,
                        });
                    }*/

                    let freightstation = RichText::new("Freight station");
                    if *uiworld.read::<Tool>() == Tool::SpecialBuilding {
                        freightstation.strong();
                    };
                    if ui.button(freightstation) {
                        *uiworld.write::<Tool>() = Tool::SpecialBuilding;

                        uiworld.write::<SpecialBuildingResource>().opt = Some(SpecialBuildKind {
                            make: Box::new(move |args, commands| {
                                let obb = args.obb;
                                let c = obb.center().z(args.mpos.z + 0.3);

                                let [offx, offy] = obb.axis().map(|x| x.normalize().z(0.0));

                                let mut tracks = vec![];

                                let pat =
                                    LanePatternBuilder::new().rail(true).one_way(true).build();

                                for i in 0..4 {
                                    tracks.push(StraightRoadGen {
                                        from: c - offx * (15.0 + i as f32 * 20.0) - offy * 100.0,
                                        to: c - offx * (15.0 + i as f32 * 20.0) + offy * 100.0,
                                        pattern: pat.clone(),
                                    });
                                }
                                commands.map_build_special_building(
                                    args.obb,
                                    BuildingKind::RailFretStation,
                                    BuildingGen::NoWalkway {
                                        door_pos: Vec2::ZERO,
                                    },
                                    tracks,
                                );
                            }),
                            w: 200.0,
                            h: 160.0,
                            asset: "rail_fret_station.glb".to_string(),
                            road_snap: false,
                        });
                    }
                });
        }

        if matches!(*uiworld.read::<Tab>(), Tab::Roadbuild | Tab::Roadcurved) {
            let rbw = 220.0;
            Window::new("Road Properties")
                .fixed_size([rbw, 380.0])
                .fixed_pos([w - rbw - toolbox_w, h * 0.5 - 30.0])
                .title_bar(true)
                .collapsible(false)
                .resizable(false)
                .show(ui, |ui| {
                    let mut roadbuild = uiworld.write::<RoadBuildResource>();
                    ui.checkbox(&mut roadbuild.snap_to_grid, "snap to grid");
                    ui.horizontal(|ui| {
                        if ui.button("zero") {
                            roadbuild.height_offset = 0.0;
                        }
                        egui::DragValue::new(&mut roadbuild.height_offset)
                            .clamp_range(0.0..=100.0)
                            .speed(1.0)
                            .ui(ui);
                        ui.label("height off");
                    });
                    let pat = &mut roadbuild.pattern_builder;

                    static BUILDERS: &[(&str, LanePatternBuilder)] = &[
                        ("Rail", LanePatternBuilder::new().rail(true)),
                        (
                            "Rail one-way",
                            LanePatternBuilder::new().rail(true).one_way(true),
                        ),
                        ("Street", LanePatternBuilder::new()),
                        ("Street one-way", LanePatternBuilder::new().one_way(true)),
                        (
                            "Avenue",
                            LanePatternBuilder::new().n_lanes(2).speed_limit(13.0),
                        ),
                        (
                            "Avenue one-way",
                            LanePatternBuilder::new()
                                .n_lanes(2)
                                .one_way(true)
                                .speed_limit(13.0),
                        ),
                        (
                            "Drive",
                            LanePatternBuilder::new()
                                .parking(false)
                                .sidewalks(false)
                                .speed_limit(13.0),
                        ),
                        (
                            "Drive one-way",
                            LanePatternBuilder::new()
                                .parking(false)
                                .sidewalks(false)
                                .one_way(true)
                                .speed_limit(13.0),
                        ),
                        (
                            "Highway",
                            LanePatternBuilder::new()
                                .n_lanes(3)
                                .speed_limit(25.0)
                                .parking(false)
                                .sidewalks(false),
                        ),
                        (
                            "Highway one-way",
                            LanePatternBuilder::new()
                                .n_lanes(3)
                                .speed_limit(25.0)
                                .parking(false)
                                .sidewalks(false)
                                .one_way(true),
                        ),
                    ];

                    let before = ui.style().spacing.interact_size;
                    ui.style_mut().spacing.interact_size = [rbw, 30.0].into();
                    for (name, lpat) in BUILDERS {
                        let text = RichText::new(name);
                        if lpat == pat {
                            text.strong();
                        }
                        if ui.button_with_size(text) {
                            *pat = *lpat;
                        }
                    }
                    ui.style_mut().spacing.interact_size = before;

                    ui.add_space(10.0);

                    if egui::CollapsingHeader::new("custom").build(ui) {
                        <LanePatternBuilder as InspectRenderStruct<LanePatternBuilder>>::render_mut(
                            pat,
                            "Road shape",
                            ui,
                            &InspectArgsStruct {
                                header: Some(false),
                                indent_children: Some(false),
                            },
                        );

                        if pat.n_lanes == 0 {
                            pat.sidewalks = true;
                            pat.parking = false;
                        }

                        if pat.n_lanes > 10 {
                            pat.n_lanes = 10;
                        }
                    }
                });
        }

        let brushes = [("Residential", LotKind::Residential)];

        if matches!(*uiworld.read::<Tab>(), Tab::Lotbrush) {
            let lbw = 130.0;
            Window::new("Lot Brush")
                .fixed_size([lbw, 50.0 + brushes.len() as f32 * 35.0])
                .fixed_pos([w - toolbox_w - lbw, h * 0.5 - 30.0])
                .hscroll(false)
                .title_bar(true)
                .collapsible(false)
                .resizable(false)
                .show(ui, |ui| {
                    let mut cur_brush = uiworld.write::<LotBrushResource>();

                    ui.style_mut().spacing.interact_size = [lbw, 35.0].into();
                    for (name, brush) in &brushes {
                        let t = RichText::new(name);
                        if std::mem::discriminant(brush) == std::mem::discriminant(&cur_brush.kind)
                        {
                            t.strong();
                        }
                        if ui.button_with_size(t) {
                            cur_brush.kind = *brush;
                        }
                    }

                    ui.horizontal(|ui| {
                        egui::DragValue::new(&mut cur_brush.radius)
                            .clamp_range(10.0..=300.0)
                            .build(ui);
                        ui.label("radius");
                    })
                });
        }

        if matches!(*uiworld.read::<Tab>(), Tab::Bulldozer) {
            let lbw = 80.0;
            Window::new("Bulldozer")
                .min_width(lbw)
                .auto_sized()
                .fixed_pos([w - toolbox_w - lbw, h * 0.5 - 30.0])
                .hscroll(false)
                .title_bar(true)
                .collapsible(false)
                .resizable(false)
                .show(ui, |ui| {
                    let mut state = uiworld.write::<BulldozerState>();
                    <BulldozerState as InspectRenderDefault<BulldozerState>>::render_mut(
                        &mut *state,
                        "Bulldozer",
                        ui,
                        &InspectArgsDefault {
                            header: Some(false),
                            indent_children: Some(false),
                            ..Default::default()
                        },
                    );
                });
        }

        let building_select_w = 160.0;
        let registry = goria.read::<GoodsCompanyRegistry>();
        let gbuildings = registry.descriptions.values().peekable();

        let iregistry = goria.read::<ItemRegistry>();

        if matches!(*uiworld.read::<Tab>(), Tab::Roadbuilding) {
            Window::new("Buildings")
                .min_width(building_select_w)
                .default_height(500.0.min(h * 0.5) as f32)
                .vscroll(true)
                .fixed_pos([w - toolbox_w - building_select_w, h * 0.5 - 30.0])
                .title_bar(true)
                .collapsible(false)
                .resizable(false)
                .show(ui, |ui| {
                    let mut cur_build = uiworld.write::<SpecialBuildingResource>();

                    let mut picked_descr = None;
                    ui.style_mut().spacing.interact_size = [building_select_w - 5.0, 35.0].into();

                    for descr in gbuildings {
                        let cur_kind = cur_build.opt.as_ref().map(|x| &*x.asset).unwrap_or("");

                        let name = RichText::new(&descr.name);
                        if descr.asset_location == cur_kind {
                            picked_descr = Some(descr);
                            name.strong();
                        };
                        if ui.button(name).clicked() || cur_build.opt.is_none() {
                            let bkind = BuildingKind::GoodsCompany(descr.id);
                            let bgen = descr.bgen;
                            cur_build.opt = Some(SpecialBuildKind {
                                road_snap: true,
                                make: Box::new(move |args, commands| {
                                    commands.map_build_special_building(
                                        args.obb,
                                        bkind,
                                        bgen,
                                        vec![],
                                    );
                                }),
                                w: descr.size,
                                h: descr.size,
                                asset: descr.asset_location.to_string(),
                            });
                        }
                    }

                    let bdescrpt_w = 180.0;

                    if let Some(descr) = picked_descr {
                        //let _tok1 = ui.push_style_var(StyleVar::WindowPadding([5.0, 5.0]));
                        //let _tok2 = ui.push_style_var(StyleVar::ItemSpacing([0.0, 3.0]));
                        Window::new("Building description")
                            .default_width(bdescrpt_w)
                            .auto_sized()
                            .fixed_pos([
                                w - toolbox_w - building_select_w - bdescrpt_w,
                                h * 0.5 - 30.0,
                            ])
                            .hscroll(false)
                            .title_bar(true)
                            .collapsible(false)
                            .resizable(false)
                            .show(ui.ctx(), |ui| {
                                ui.label(format!("workers: {}", descr.n_workers));
                                ui.add_space(10.0);
                                if !descr.recipe.consumption.is_empty() {
                                    ui.label("consumption:");
                                    for (kind, n) in &descr.recipe.consumption {
                                        ui.label(format!("- {} x{}", &iregistry[*kind].label, n));
                                    }
                                    ui.add_space(10.0);
                                }
                                if !descr.recipe.production.is_empty() {
                                    ui.label("production:");
                                    for (kind, n) in &descr.recipe.production {
                                        ui.label(format!("- {} x{}", &iregistry[*kind].label, n));
                                    }
                                    ui.add_space(10.0);
                                }
                                ui.label(format!("time: {}s", descr.recipe.complexity));
                                ui.label(format!(
                                    "storage multiplier: {}",
                                    descr.recipe.storage_multiplier
                                ));
                            });
                    }
                });
        }
    }

    pub(crate) fn inspector(ui: &Context, uiworld: &mut UiWorld, goria: &Egregoria) {
        let mut inspected = *uiworld.read::<InspectedEntity>();
        let e = unwrap_or!(inspected.e, return);

        let mut is_open = true;
        Window::new("Inspect")
            .size([300.0, 300.0], egui::Condition::Appearing)
            .position([30.0, 160.0], egui::Condition::Appearing)
            .opened(&mut is_open)
            .build(ui, || {
                let mut ins = crate::gui::inspect::InspectRenderer { entity: e };
                ins.render(uiworld, goria, ui);
                inspected.e = Some(ins.entity);
            });
        if !is_open {
            inspected.e = None;
        }
        *uiworld.write::<InspectedEntity>() = inspected;
    }

    pub(crate) fn time_controls(&mut self, ui: &Context, uiworld: &mut UiWorld, goria: &Egregoria) {
        let time = goria.read::<GameTime>().daytime;
        let warp = &mut uiworld.write::<Settings>().time_warp;
        let depause_warp = &mut self.depause_warp;
        if uiworld
            .read::<KeyboardInfo>()
            .just_pressed
            .contains(&KeyCode::Space)
        {
            if *warp == 0 {
                *warp = *depause_warp;
            } else {
                *depause_warp = *warp;
                *warp = 0;
            }
        }

        let [_, h] = ui.io().display_size;
        let _tok1 = ui.push_style_var(StyleVar::WindowRounding(0.0));
        let _tok2 = ui.push_style_var(StyleVar::ItemSpacing([10.0, 7.0]));
        Window::new("Time controls")
            .size([165.0, 55.0], egui::Condition::Always)
            .position([-1.0, h - 52.0], egui::Condition::Always)
            .no_decoration()
            .collapsible(false)
            .resizable(false)
            .build(ui, || {
                ui.label(format!(" Day {}", time.day));

                ui.same_line_with_pos(115.0);

                ui.label(format!("{:02}:{:02}", time.hour, time.second));

                let red = ui.push_style_color(StyleColor::Header, [0.7, 0.2, 0.2, 0.5]);

                if egui::Selectable::new("   ||")
                    .size([29.0, 15.0])
                    .selected(*warp == 0)
                    .build(ui)
                {
                    *depause_warp = *warp;
                    *warp = 0;
                }

                red.pop();

                ui.same_line();

                if egui::Selectable::new("  1x")
                    .size([27.0, 15.0])
                    .selected(*warp == 1)
                    .build(ui)
                {
                    *warp = 1;
                }

                ui.same_line();

                if egui::Selectable::new("  3x")
                    .size([27.0, 15.0])
                    .selected(*warp == 3)
                    .build(ui)
                {
                    *warp = 3;
                }

                ui.same_line();

                if egui::Selectable::new(" Max")
                    .size([33.0, 15.0])
                    .selected(*warp == 1000)
                    .build(ui)
                {
                    *warp = 1000;
                }
            });
    }

    pub(crate) fn menu_bar(&mut self, ui: &Context, uiworld: &mut UiWorld, goria: &Egregoria) {
        let _t = ui.push_style_var(StyleVar::ItemSpacing([3.0, 0.0]));

        ui.main_menu_bar(|| {
            self.windows.menu(ui);

            let [w, h] = ui.window_size();

            let mut name = "Save";
            let mut tok = None;
            if uiworld.saving_status.load(Ordering::SeqCst) {
                name = "Saving...";
                tok = Some(ui.begin_disabled(true));
            }

            if ui.button_with_size(name, [80.0, h]) {
                uiworld.please_save = true;
                uiworld.save_to_disk();
            }
            drop(tok);

            ui.label(format!("Money: {}", goria.read::<Government>().money));

            let mut estate = uiworld.write::<ExitState>();
            let mut please_save = uiworld.please_save;

            match *estate {
                ExitState::NoExit => {}
                ExitState::ExitAsk | ExitState::Saving => {
                    let [w, h] = ui.io().display_size;
                    Window::new("Exit Menu")
                        .position([w * 0.5, h * 0.5], Condition::Appearing)
                        .always_auto_resize(true)
                        .position_pivot([0.5, 0.5])
                        .build(ui, || {
                            let _tok = ui.push_style_var(StyleVar::ItemSpacing([2.0, 5.0]));
                            if let ExitState::Saving = *estate {
                                ui.label("Saving...");
                                if !uiworld.please_save
                                    && !uiworld.saving_status.load(Ordering::SeqCst)
                                {
                                    std::process::exit(0);
                                }
                                return;
                            }
                            if ui.button("Save and exit") {
                                if let ExitState::ExitAsk = *estate {
                                    please_save = true;
                                    *estate = ExitState::Saving;
                                }
                            }
                            if ui.button("Exit") {
                                std::process::exit(0);
                            }
                            if ui.button("Cancel") {
                                *estate = ExitState::NoExit;
                            }
                        });
                }
            }

            {
                let off = if matches!(*estate, ExitState::ExitAsk) {
                    110.0
                } else {
                    65.0
                };
                let _red = ui.push_style_color(StyleColor::Button, [0.7, 0.3, 0.3, 1.0]);
                ui.same_line_with_pos(w - off);

                match *estate {
                    ExitState::NoExit => {
                        if ui.button_with_size("Exit", [50.0, h]) {
                            *estate = ExitState::ExitAsk;
                        }
                    }
                    ExitState::ExitAsk => {
                        if ui.button("Save and exit") {
                            if let ExitState::ExitAsk = *estate {
                                please_save = true;
                                *estate = ExitState::Saving;
                            }
                        }
                    }
                    ExitState::Saving => {
                        ui.label("Saving...");
                    }
                }
            }
            drop(estate);
            uiworld.please_save = please_save;
        });
    }
}

pub(crate) enum ExitState {
    NoExit,
    ExitAsk,
    Saving,
}

impl Default for ExitState {
    fn default() -> Self {
        Self::NoExit
    }
}
