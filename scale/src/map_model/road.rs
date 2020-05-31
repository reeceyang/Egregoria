use crate::geometry::polyline::PolyLine;
use crate::geometry::splines::Spline;
use crate::geometry::Vec2;
use crate::map_model::{
    IntersectionID, Intersections, Lane, LaneDirection, LaneID, LaneKind, LanePattern, Lanes,
    Roads, TrafficControl,
};
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

#[derive(Serialize, Deserialize)]
pub struct ParkingSpot {
    pub parent: LaneID,
    pub dist_along: f32,
}

new_key_type! {
    pub struct RoadID;
}

#[derive(Serialize, Deserialize)]
pub struct Road {
    pub id: RoadID,
    pub src: IntersectionID,
    pub dst: IntersectionID,

    major_interpolation_points: PolyLine,
    pub length: f32,
    pub width: f32,

    pub src_interface: f32,
    pub dst_interface: f32,

    lanes_forward: Vec<LaneID>,
    lanes_backward: Vec<LaneID>,

    pub parking_spots: Vec<ParkingSpot>,

    pub lane_pattern: LanePattern,
}

impl Road {
    /// Builds the road and its associated lanes
    pub fn make(
        src: IntersectionID,
        dst: IntersectionID,
        interpolation_points: PolyLine,
        lane_pattern: LanePattern,
        intersections: &Intersections,
        lanes: &mut Lanes,
        store: &mut Roads,
    ) -> RoadID {
        let id = store.insert_with_key(|id| Self {
            id,
            src,
            dst,
            src_interface: 9.0,
            dst_interface: 9.0,
            major_interpolation_points: interpolation_points,
            width: 1.0,
            length: 1.0,
            lanes_forward: vec![],
            lanes_backward: vec![],
            lane_pattern: lane_pattern.clone(),
            parking_spots: vec![],
        });
        let road = &mut store[id];
        for lane in &lane_pattern.lanes_forward {
            road.add_lane(lanes, *lane, LaneDirection::Forward);
        }
        for lane in &lane_pattern.lanes_backward {
            road.add_lane(lanes, *lane, LaneDirection::Backward);
        }
        road.gen_pos(intersections, lanes);
        id
    }

    pub fn is_one_way(&self) -> bool {
        self.lanes_forward.is_empty() || self.lanes_backward.is_empty()
    }

    pub fn n_lanes(&self) -> usize {
        self.lanes_backward.len() + self.lanes_forward.len()
    }

    pub fn lanes_iter(&self) -> impl Iterator<Item = &LaneID> {
        self.lanes_forward
            .iter()
            .rev()
            .chain(self.lanes_backward.iter())
    }

    pub fn sidewalks<'a>(
        &self,
        from: IntersectionID,
        lanes: &'a Lanes,
    ) -> (Option<&'a Lane>, Option<&'a Lane>) {
        (
            self.incoming_lanes_to(from)
                .iter()
                .map(|x| &lanes[*x])
                .find(|x| matches!(x.kind, LaneKind::Walking)),
            self.outgoing_lanes_from(from)
                .iter()
                .map(|x| &lanes[*x])
                .find(|x| matches!(x.kind, LaneKind::Walking)),
        )
    }

    pub fn add_lane(
        &mut self,
        store: &mut Lanes,
        lane_type: LaneKind,
        direction: LaneDirection,
    ) -> LaneID {
        let src_dir = self.interpolation_points().first_dir().unwrap();
        let dst_dir = -self.interpolation_points().last_dir().unwrap();

        let (src, dst, src_dir, dst_dir) = match direction {
            LaneDirection::Forward => (self.src, self.dst, src_dir, dst_dir),
            LaneDirection::Backward => (self.dst, self.src, dst_dir, src_dir),
        };
        let id = store.insert_with_key(|id| Lane {
            id,
            src,
            dst,
            src_dir,
            dst_dir,
            kind: lane_type,
            points: Default::default(),
            width: lane_type.width(),
            inter_length: self.length,
            control: TrafficControl::Always,
        });
        match direction {
            LaneDirection::Forward => self.lanes_forward.push(id),
            LaneDirection::Backward => self.lanes_backward.push(id),
        }
        id
    }

    pub fn gen_pos(&mut self, intersections: &Intersections, lanes: &mut Lanes) {
        *self.major_interpolation_points.first_mut().unwrap() = intersections[self.src].pos;
        *self.major_interpolation_points.last_mut().unwrap() = intersections[self.dst].pos;
        self.length = self.major_interpolation_points.length();

        self.width = self.lanes_iter().map(|&x| lanes[x].width).sum();

        let mut dist_from_bottom = 0.0;
        for &id in self.lanes_iter() {
            let l = &mut lanes[id];
            l.gen_pos(intersections, self, dist_from_bottom);
            dist_from_bottom += l.width;
        }
    }

    pub fn interpolation_spline(&self) -> Spline {
        let p = &self.major_interpolation_points;
        let l = p.n_points();
        let from = p[0] + self.orientation_from(self.src) * self.interface_from(self.src);
        let to = p[l - 1] + self.orientation_from(self.dst) * self.interface_from(self.dst);

        Spline {
            from,
            to,
            from_derivative: (p[1] - from) * std::f32::consts::FRAC_1_SQRT_2,
            to_derivative: (to - p[l - 2]) * std::f32::consts::FRAC_1_SQRT_2,
        }
    }

    pub fn interface_from(&self, id: IntersectionID) -> f32 {
        let (my_interf, other_interf) = self.interfaces_from(id);

        let l = self.length - 1.0;
        let half = l * 0.5;

        if my_interf + other_interf > l {
            if my_interf > half && other_interf > half {
                half
            } else if my_interf > half {
                l - other_interf
            } else {
                my_interf
            }
        } else {
            my_interf
        }
    }

    fn interfaces_from(&self, id: IntersectionID) -> (f32, f32) {
        if id == self.src {
            (self.src_interface, self.dst_interface)
        } else if id == self.dst {
            (self.dst_interface, self.src_interface)
        } else {
            panic!("Asking interface from from an intersection not conected to the road");
        }
    }

    pub fn set_interface(&mut self, id: IntersectionID, v: f32) {
        if id == self.src {
            self.src_interface = v;
        } else if id == self.dst {
            self.dst_interface = v;
        } else {
            panic!("Setting interface from from an intersection not conected to the road");
        }
    }

    pub fn max_interface(&mut self, id: IntersectionID, v: f32) {
        if id == self.src {
            self.src_interface = self.src_interface.max(v);
        } else if id == self.dst {
            self.dst_interface = self.dst_interface.max(v);
        } else {
            panic!("Setting interface from from an intersection not conected to the road");
        }
    }

    pub fn orientation_from(&self, id: IntersectionID) -> Vec2 {
        if id == self.src {
            self.major_interpolation_points.first_dir().unwrap()
        } else if id == self.dst {
            -self.major_interpolation_points.last_dir().unwrap()
        } else {
            panic!("Asking dir from from an intersection not conected to the road");
        }
    }

    pub fn incoming_lanes_to(&self, id: IntersectionID) -> &Vec<LaneID> {
        if id == self.src {
            &self.lanes_backward
        } else if id == self.dst {
            &self.lanes_forward
        } else {
            panic!("Asking incoming lanes from from an intersection not conected to the road");
        }
    }

    pub fn outgoing_lanes_from(&self, id: IntersectionID) -> &Vec<LaneID> {
        if id == self.src {
            &self.lanes_forward
        } else if id == self.dst {
            &self.lanes_backward
        } else {
            panic!("Asking outgoing lanes from from an intersection not conected to the road");
        }
    }

    pub fn interpolation_points(&self) -> &PolyLine {
        &self.major_interpolation_points
    }

    pub fn interpolation_points_owned(self) -> PolyLine {
        self.major_interpolation_points
    }

    pub fn src_point(&self) -> Vec2 {
        self.major_interpolation_points.first().unwrap()
    }

    pub fn dst_point(&self) -> Vec2 {
        self.major_interpolation_points.last().unwrap()
    }

    pub fn other_end(&self, my_end: IntersectionID) -> IntersectionID {
        if self.src == my_end {
            return self.dst;
        } else if self.dst == my_end {
            return self.src;
        }
        panic!(
            "Asking other end of {:?} which isn't connected to {:?}",
            self.id, my_end
        );
    }
}
