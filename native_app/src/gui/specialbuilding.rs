use super::Tool;
use crate::gui::{ErrorTooltip, InspectedBuilding, PotentialCommands};
use crate::inputmap::{InputAction, InputMap};
use crate::rendering::immediate::{ImmediateDraw, ImmediateSound};
use crate::uiworld::UiWorld;
use common::AudioKind;
use geom::{Degrees, Intersect, Vec3, OBB};
use ordered_float::OrderedFloat;
use simulation::map::{ProjectFilter, ProjectKind};
use simulation::world_command::WorldCommand;
use simulation::Simulation;
use std::borrow::Cow;

pub struct SpecialBuildArgs {
    pub obb: OBB,
    pub mpos: Vec3,
}

pub struct SpecialBuildKind {
    pub make: Box<dyn Fn(&SpecialBuildArgs) -> Vec<WorldCommand> + Send + Sync + 'static>,
    pub w: f32,
    pub h: f32,
    pub asset: String,
    pub road_snap: bool,
}

#[derive(Default)]
pub struct SpecialBuildingResource {
    pub opt: Option<SpecialBuildKind>,
    pub last_obb: Option<OBB>,
    pub rotation: Degrees,
}

/// SpecialBuilding tool
/// Allows to build special buildings like farms, factories, etc.
pub fn specialbuilding(sim: &Simulation, uiworld: &mut UiWorld) {
    profiling::scope!("gui::specialbuilding");
    let mut state = uiworld.write::<SpecialBuildingResource>();
    let tool = *uiworld.read::<Tool>();
    let inp = uiworld.read::<InputMap>();
    let mut draw = uiworld.write::<ImmediateDraw>();
    let mut sound = uiworld.write::<ImmediateSound>();

    let map = sim.map();

    let commands = &mut *uiworld.commands();

    if !matches!(tool, Tool::SpecialBuilding) {
        return;
    }

    for command in uiworld.received_commands().iter() {
        if let WorldCommand::MapBuildSpecialBuilding { pos, kind, .. } = command {
            if let Some(ProjectKind::Building(bid)) = map
                .spatial_map()
                .query(pos.center(), ProjectFilter::BUILDING)
                .next()
            {
                if let Some(b) = map.buildings().get(bid) {
                    if b.kind == *kind {
                        uiworld.write::<InspectedBuilding>().e = Some(bid);
                    }
                }
            }
        }
    }

    if inp.act.contains(&InputAction::Rotate) {
        state.rotation += Degrees(inp.wheel);
        state.rotation.normalize();
    }

    let SpecialBuildKind {
        w,
        h,
        ref asset,
        ref make,
        road_snap,
    } = *unwrap_or!(&state.opt, return);

    let mpos = unwrap_ret!(inp.unprojected);
    let roads = map.roads();

    let diag = 0.5 * w.hypot(h);
    let hover_obb = OBB::new(mpos.xy(), state.rotation.vec2(), w, h);

    let mut draw = |obb, red| {
        let p = asset.to_string();
        let col = if red {
            simulation::config().special_building_invalid_col
        } else {
            simulation::config().special_building_col
        };

        if p.ends_with(".png") || p.ends_with(".jpg") {
            draw.textured_obb(obb, p, mpos.z + 0.1).color(col);
        } else if p.ends_with(".glb") {
            draw.mesh(p, obb.center().z(mpos.z), obb.axis()[0].normalize().z0())
                .color(col);
        }
    };

    let mut rid = None;
    let mut obb = hover_obb;

    if road_snap {
        let closest_road = map
            .spatial_map()
            .query_around(mpos.xy(), diag, ProjectFilter::ROAD)
            .filter_map(|x| match x {
                ProjectKind::Road(id) => Some(&roads[id]),
                _ => None,
            })
            .min_by_key(move |p| OrderedFloat(p.points().project_dist2(mpos)));
        let Some(closest_road) = closest_road else {
            *uiworld.write::<ErrorTooltip>() = ErrorTooltip::new(Cow::Borrowed("No road nearby"));
            return draw(hover_obb, true);
        };

        let (proj, _, dir) = closest_road.points().project_segment_dir(mpos);
        let dir = dir.xy();

        if !proj.is_close(mpos, diag + closest_road.width * 0.5) {
            *uiworld.write::<ErrorTooltip>() = ErrorTooltip::new(Cow::Borrowed("No road nearby"));
            return draw(hover_obb, true);
        }

        let side = if (mpos.xy() - proj.xy()).dot(dir.perpendicular()) > 0.0 {
            dir.perpendicular()
        } else {
            -dir.perpendicular()
        };

        let first = closest_road.points().first();
        let last = closest_road.points().last();

        obb = OBB::new(
            proj.xy() + side * (h + closest_road.width + 0.5) * 0.5,
            side,
            w,
            h,
        );

        if proj.distance(first) < diag || proj.distance(last) < diag {
            *uiworld.write::<ErrorTooltip>() =
                ErrorTooltip::new(Cow::Borrowed("Too close to side"));
            draw(obb, true);
            return;
        }

        if closest_road.sidewalks(closest_road.src).incoming.is_none() {
            *uiworld.write::<ErrorTooltip>() =
                ErrorTooltip::new(Cow::Borrowed("Sidewalk required"));
            draw(obb, true);
            return;
        }

        rid = Some(closest_road.id);
    }

    if map
        .spatial_map()
        .query(
            obb,
            ProjectFilter::ROAD | ProjectFilter::INTER | ProjectFilter::BUILDING,
        )
        .any(|x| {
            if let Some(rid) = rid {
                ProjectKind::Road(rid) != x
            } else {
                true
            }
        })
        || state.last_obb.map(|x| x.intersects(&obb)).unwrap_or(false)
    {
        *uiworld.write::<ErrorTooltip>() =
            ErrorTooltip::new(Cow::Borrowed("Intersecting with something"));
        draw(obb, true);
        return;
    }

    draw(obb, false);

    let cmds: Vec<WorldCommand> = make(&SpecialBuildArgs { obb, mpos });
    if inp.act.contains(&InputAction::Select) {
        commands.extend(cmds);
        sound.play("road_lay", AudioKind::Ui);
        state.last_obb = Some(obb);
    } else if let Some(last) = cmds.last() {
        uiworld.write::<PotentialCommands>().set(last.clone());
    }
}
