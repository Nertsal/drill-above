use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, geng::Assets)]
#[asset(json)]
pub struct Rules {
    pub gravity: vec2<Coord>,
    pub move_speed: Coord,
    pub full_control_acc: Coord,
    pub low_control_acc: Coord,

    pub jump_buffer_time: Time,
    pub coyote_time: Time,
    pub edge_correction_max: Coord,

    pub free_fall_speed: Coord,
    pub wall_slide_speed: Coord,

    pub jump: JumpRules,
    pub drill: DrillRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpRules {
    pub normal_strength: Coord,
    pub wall_strength: Coord,
    pub wall_angle: R32,
    pub wall_timeout: Time,
    pub fall_multiplier: Coord,
    pub low_multiplier: Coord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrillRules {
    pub can_dash: bool,
    pub release_time: Time,
    pub speed_min: Coord,
    pub speed_inc: Coord,
    pub dash_time: Time,
    pub dash_speed_min: Coord,
    pub dash_speed_inc: Coord,
    pub jump_speed_min: Coord,
    pub jump_speed_inc: Coord,
}
