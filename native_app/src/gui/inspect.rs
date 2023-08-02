use crate::gui::follow::FollowEntity;
use crate::uiworld::UiWorld;
use egregoria::economy::{ItemRegistry, Market};
use egregoria::transportation::Location;
use egregoria::{
    AnyEntity, CompanyEnt, Egregoria, FreightStationEnt, HumanEnt, SoulID, TrainEnt, VehicleEnt,
    WagonEnt,
};
use egui::Ui;
use egui_inspect::{Inspect, InspectArgs};

/// Inspect window
/// Allows to inspect an entity
pub(crate) struct InspectRenderer {
    pub(crate) entity: AnyEntity,
}

impl InspectRenderer {
    pub(crate) fn render(&mut self, uiworld: &mut UiWorld, goria: &Egregoria, ui: &mut Ui) {
        let entity = self.entity;
        ui.label(format!("{:?}", self.entity));

        let args = InspectArgs {
            indent_children: Some(false),
            ..Default::default()
        };

        match entity {
            AnyEntity::VehicleID(x) => {
                <VehicleEnt as Inspect<VehicleEnt>>::render(goria.get(x).unwrap(), "", ui, &args)
            }
            AnyEntity::TrainID(x) => {
                <TrainEnt as Inspect<TrainEnt>>::render(goria.get(x).unwrap(), "", ui, &args)
            }
            AnyEntity::WagonID(x) => {
                <WagonEnt as Inspect<WagonEnt>>::render(goria.get(x).unwrap(), "", ui, &args)
            }
            AnyEntity::FreightStationID(x) => {
                <FreightStationEnt as Inspect<FreightStationEnt>>::render(
                    goria.get(x).unwrap(),
                    "",
                    ui,
                    &args,
                )
            }
            AnyEntity::CompanyID(x) => {
                <CompanyEnt as Inspect<CompanyEnt>>::render(goria.get(x).unwrap(), "", ui, &args)
            }
            AnyEntity::HumanID(x) => {
                <HumanEnt as Inspect<HumanEnt>>::render(goria.get(x).unwrap(), "", ui, &args)
            }
        }

        if let AnyEntity::VehicleID(id) = entity {
            for (hid, h) in goria.world().humans.iter() {
                if h.location == Location::Vehicle(id)
                    && ui
                        .small_button(&*format!("inspect inside vehicle: {hid:?}"))
                        .clicked()
                {
                    self.entity = hid.into();
                    return;
                }
            }
        }

        /*
        if let Some(coll) = goria.comp::<Collider>(self.entity) {
            if let Some((pos, po)) = goria.read::<CollisionWorld>().get(coll.0) {
                egui::CollapsingHeader::new("Physics Object").show(ui, |ui| {
                    <Vec2 as Inspect<Vec2>>::render(&pos, "pos", ui, &InspectArgs::default());
                    <PhysicsObject as Inspect<PhysicsObject>>::render(
                        po,
                        "aaaa",
                        ui,
                        &InspectArgs {
                            header: Some(false),
                            indent_children: Some(false),
                            min_value: None,
                            max_value: None,
                            step: None,
                        },
                    )
                });
            } else {
                ui.label(
                    RichText::new("Invalid coll handle")
                        .color(Color32::from_rgba_unmultiplied(255, 0, 0, 255)),
                );
            }
        }*/

        {
            let mut follow = uiworld.write::<FollowEntity>();
            follow.update(ui, entity);
        }

        if let Ok(soul) = SoulID::try_from(entity) {
            let market = goria.read::<Market>();
            let registry = goria.read::<ItemRegistry>();
            let mut capitals = vec![];
            let mut borders = vec![];
            let mut sellorders = vec![];
            for (kind, market) in market.inner() {
                let cap = unwrap_or!(market.capital(soul), continue);
                capitals.push((kind, cap));
                if let Some(b) = market.buy_order(soul) {
                    borders.push((kind, b));
                }
                if let Some(s) = market.sell_order(soul) {
                    sellorders.push((kind, s));
                }
            }

            if !capitals.is_empty() {
                egui::CollapsingHeader::new("Capital").show(ui, |ui| {
                    ui.columns(2, |ui| {
                        for (kind, cap) in capitals {
                            ui[0].label(&registry[*kind].label);
                            ui[1].label(format!("{cap}"));
                        }
                    });
                });
            }

            if !borders.is_empty() {
                egui::CollapsingHeader::new("Buy orders").show(ui, |ui| {
                    ui.columns(2, |ui| {
                        for (kind, b) in borders {
                            ui[0].label(&registry[*kind].label);
                            ui[1].label(format!("{b:#?}"));
                        }
                    });
                });
            }

            if !sellorders.is_empty() {
                egui::CollapsingHeader::new("Sell orders").show(ui, |ui| {
                    ui.columns(2, |ui| {
                        for (kind, b) in sellorders {
                            ui[0].label(&registry[*kind].label);
                            ui[1].label(format!("{b:#?}"));
                        }
                    });
                });
            }
        }
    }
}
