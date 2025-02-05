use crate::uiworld::UiWorld;
use egui::{Context, Ui, Widget};
use simulation::economy::{ItemRegistry, Market};
use simulation::world_command::WorldCommand;
use simulation::{Simulation, SoulID};

use crate::gui::inspect::entity_link;
use crate::gui::item_icon;
use egui_inspect::{Inspect, InspectArgs, InspectVec2Rotation};
use simulation::map::{Building, BuildingID, BuildingKind, Zone, MAX_ZONE_AREA};
use simulation::map_dynamic::BuildingInfos;
use simulation::souls::freight_station::FreightTrainState;
use simulation::souls::goods_company::{GoodsCompanyRegistry, Recipe};

/// Inspect a specific building, showing useful information about it
pub fn inspect_building(uiworld: &mut UiWorld, sim: &Simulation, ui: &Context, id: BuildingID) {
    let map = sim.map();
    let Some(building) = map.buildings().get(id) else {
        return;
    };
    let gregistry = sim.read::<GoodsCompanyRegistry>();

    let title: &str = match building.kind {
        BuildingKind::House => "House",
        BuildingKind::GoodsCompany(id) => &gregistry.descriptions[id].name,
        BuildingKind::RailFreightStation => "Rail Freight Station",
        BuildingKind::TrainStation => "Train Station",
        BuildingKind::ExternalTrading => "External Trading",
    };

    egui::Window::new(title)
        .resizable(false)
        .auto_sized()
        .show(ui, |ui| {
            if cfg!(debug_assertions) {
                ui.label(format!("{:?}", building.id));
            }

            match building.kind {
                BuildingKind::House => render_house(ui, uiworld, sim, building),
                BuildingKind::GoodsCompany(_) => {
                    render_goodscompany(ui, uiworld, sim, building);
                }
                BuildingKind::RailFreightStation => {
                    render_freightstation(ui, uiworld, sim, building);
                }
                BuildingKind::TrainStation => {}
                BuildingKind::ExternalTrading => {}
            };

            if let Some(ref zone) = building.zone {
                let mut cpy = zone.filldir;
                if InspectVec2Rotation::render_mut(
                    &mut cpy,
                    "fill angle",
                    ui,
                    &InspectArgs::default(),
                ) {
                    uiworld.commands().push(WorldCommand::UpdateZone {
                        building: id,
                        zone: Zone {
                            filldir: cpy,
                            ..zone.clone()
                        },
                    })
                }
                egui::ProgressBar::new(zone.area / MAX_ZONE_AREA)
                    .text(format!("area: {}/{}", zone.area, MAX_ZONE_AREA))
                    .desired_width(200.0)
                    .ui(ui);
            }
        });
}

fn render_house(ui: &mut Ui, uiworld: &mut UiWorld, sim: &Simulation, b: &Building) {
    let binfos = sim.read::<BuildingInfos>();
    let Some(info) = binfos.get(b.id) else {
        return;
    };
    let Some(SoulID::Human(owner)) = info.owner else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label("Owner");
        entity_link(uiworld, sim, ui, owner);
    });

    ui.label("Currently in the house:");
    for &soul in info.inside.iter() {
        let SoulID::Human(soul) = soul else {
            continue;
        };
        entity_link(uiworld, sim, ui, soul);
    }
}

fn render_freightstation(ui: &mut Ui, uiworld: &mut UiWorld, sim: &Simulation, b: &Building) {
    let Some(SoulID::FreightStation(owner)) = sim.read::<BuildingInfos>().owner(b.id) else {
        return;
    };
    let Some(freight) = sim.world().get(owner) else {
        return;
    };

    ui.label(format!("Waiting cargo: {}", freight.f.waiting_cargo));
    ui.label(format!("Wanted cargo: {}", freight.f.wanted_cargo));

    ui.add_space(10.0);
    ui.label("Trains:");
    for (tid, state) in &freight.f.trains {
        ui.horizontal(|ui| {
            entity_link(uiworld, sim, ui, *tid);
            match state {
                FreightTrainState::Arriving => {
                    ui.label("Arriving");
                }
                FreightTrainState::Loading => {
                    ui.label("Loading");
                }
                FreightTrainState::Moving => {
                    ui.label("Moving");
                }
            }
        });
    }
}

fn render_goodscompany(ui: &mut Ui, uiworld: &mut UiWorld, sim: &Simulation, b: &Building) {
    let owner = sim.read::<BuildingInfos>().owner(b.id);

    let Some(SoulID::GoodsCompany(c_id)) = owner else {
        return;
    };
    let Some(c) = sim.world().companies.get(c_id) else {
        return;
    };
    let goods = &c.comp;
    let workers = &c.workers;

    let market = sim.read::<Market>();
    let itemregistry = sim.read::<ItemRegistry>();
    let max_workers = goods.max_workers;
    egui::ProgressBar::new(workers.0.len() as f32 / max_workers as f32)
        .text(format!("workers: {}/{}", workers.0.len(), max_workers))
        .desired_width(200.0)
        .ui(ui);
    if let Some(driver) = goods.driver {
        ui.horizontal(|ui| {
            ui.label("Driver is");
            entity_link(uiworld, sim, ui, driver);
        });
    }
    let productivity = goods.productivity(workers.0.len(), b.zone.as_ref());
    let productivity = (productivity * 100.0).round();
    if productivity < 100.0 {
        egui::ProgressBar::new(productivity)
            .text(format!("productivity: {productivity:.0}%"))
            .desired_width(200.0)
            .ui(ui);
    }

    render_recipe(ui, uiworld, sim, &goods.recipe);

    egui::ProgressBar::new(goods.progress)
        .show_percentage()
        .desired_width(200.0)
        .ui(ui);

    ui.add_space(10.0);
    ui.label("Storage");

    let jobopening = itemregistry.id("job-opening");
    for (&id, m) in market.iter() {
        let Some(v) = m.capital(c_id.into()) else {
            continue;
        };
        if id == jobopening && v == 0 {
            continue;
        }
        let Some(item) = itemregistry.get(id) else {
            continue;
        };

        item_icon(ui, uiworld, item, v);
    }
}

fn render_recipe(ui: &mut Ui, uiworld: &UiWorld, sim: &Simulation, recipe: &Recipe) {
    let registry = sim.read::<ItemRegistry>();

    if recipe.consumption.is_empty() {
        ui.label("No Inputs");
    } else {
        ui.label(if recipe.consumption.len() == 1 {
            "Input"
        } else {
            "Inputs"
        });
        ui.horizontal(|ui| {
            for &(good, amount) in recipe.consumption.iter() {
                let Some(item) = registry.get(good) else {
                    continue;
                };
                item_icon(ui, uiworld, item, amount);
            }
        });
    }

    if recipe.production.is_empty() {
        ui.label("No Outputs");
    } else {
        ui.label(if recipe.production.len() == 1 {
            "Output"
        } else {
            "Outputs"
        });
        ui.horizontal(|ui| {
            for &(good, amount) in recipe.production.iter() {
                let Some(item) = registry.get(good) else {
                    continue;
                };
                item_icon(ui, uiworld, item, amount);
            }
        });
    }
}
