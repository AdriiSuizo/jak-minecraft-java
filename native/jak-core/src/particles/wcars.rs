//! Original `wcar.gc` data. Coordinates are converted once, from GOAL to metres.
use super::{keyword, parse, Sexp};
use glam::Vec3;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Wheel {
    pub anchor: Vec3, pub radius: f32, pub arm: f32, pub steering_arm: f32,
    pub scale: f32, pub travel: f32, pub probe_offset: f32, pub settle: f32,
    pub spring: f32, pub damping: f32, pub camber: f32,
    pub longitudinal: bool, pub wheel_follows_arm: bool,
}
impl Wheel {
    // Toad overrides wvehicle-method-164 with v-toad-method-203: its front
    // arms swing longitudinally about (2.6, .65, 1.86), not sideways.
    pub fn vertical_travel(&self) -> f32 {
        if self.longitudinal { self.travel*(1.-(self.travel/(2.*3.318)).powi(2)).sqrt() } else {self.travel}
    }
    pub fn suspension_angle(&self, extension:f32)->f32 {
        let sine=if self.longitudinal {extension*self.vertical_travel()/3.318}
            else {(self.probe_offset+self.radius*self.scale-extension*self.travel)/self.arm};
        sine.clamp(-1.,1.).asin()
    }
    pub fn center(&self, extension: f32, side: f32, steer: f32) -> Vec3 {
        let a=self.suspension_angle(extension);
        let hub=if self.longitudinal {Vec3::new(2.6*side,0.65-3.318*a.sin(),1.86+3.318*a.cos())}
            else {self.anchor*Vec3::new(side,1.,1.)+Vec3::new(side*self.arm*a.cos(),self.arm*a.sin(),0.)};
        hub+Vec3::new(side*self.steering_arm*steer.cos(),0.,-side*self.steering_arm*steer.sin())
    }
}

#[derive(Clone, Debug)]
pub struct Specs {
    pub mass: f32, pub hp: f32, pub toughness: f32, pub gravity: f32,
    pub thrust: f32, pub response: f32, pub speed: f32, pub steer: f32,
    pub grip: f32, pub sliding_grip: f32, pub jump: f32,
    pub boost: f32, pub boost_duration: f32, pub height: f32,
    pub seat: Vec3, pub wheels: [Wheel;2],
    pub idle_rpm: f32, pub max_rpm: f32, pub peak_rpm: f32,
    pub upshift: f32, pub downshift: f32, pub final_drive: f32, pub gears: Vec<f32>,
    pub audio_idle_rpm:f32,pub audio_engine_rpm:f32,pub idle_pitch_scale:f32,pub engine_pitch_scale:f32,pub idle_crossover:f32,pub engine_crossover:f32,
    pub sounds: HashMap<String,String>, pub points: HashMap<String,Vec<Vec3>>,
}
fn list(v: Option<&Sexp>) -> &[Sexp] { match v {Some(Sexp::List(v))=>v,_=>&[]} }
fn num(v: &[Sexp], k: &str) -> f32 {
    match keyword(v,k) {
        Some(Sexp::List(x)) if x.first().and_then(Sexp::symbol)==Some("meters") => x.get(1).and_then(Sexp::number).unwrap_or(0.)*4096.,
        Some(Sexp::List(x)) if x.first().and_then(Sexp::symbol)==Some("seconds") => x.get(1).and_then(Sexp::number).unwrap_or(0.),
        Some(x)=>x.number().unwrap_or(0.), _=>0.
    }
}
fn vec(v: &[Sexp]) -> Vec3 { Vec3::new(num(v,":x"),num(v,":y"),num(v,":z"))/4096. }
pub fn read(source: &str) -> HashMap<String,Specs> {
    let mut out=HashMap::new();
    for form in parse(source) {
        let f=list(Some(&form));
        if f.first().and_then(Sexp::symbol)!=Some("define") {continue}
        let name=match f.get(1).and_then(Sexp::symbol) {Some("*v-snake-constants*")=>"snake",Some("*v-toad-constants*")=>"toad",_=>continue};
        let b=list(f.get(2)); let h=list(keyword(b,":handling")); let p=list(keyword(b,":physics-model"));
        let e=list(keyword(b,":engine")); let t=list(keyword(b,":transmission")); let d=list(keyword(b,":damage"));
        let wheel=|key| {let w=list(keyword(p,key)); Wheel {
            longitudinal:name=="toad"&&key==":front-wheel",
            wheel_follows_arm:list(keyword(w,":flags")).iter().any(|v|v.symbol()==Some("vwf5")),
            anchor:vec(list(keyword(w,":local-pos"))),radius:num(w,":radius")/4096.,arm:num(w,":susp-arm-length")/4096.,
            steering_arm:num(w,":steer-arm-length")/4096.,scale:num(w,":scale"),travel:num(w,":travel")/4096.,
            probe_offset:num(w,":probe-y-offset")/4096.,settle:num(w,":settle-pos"),spring:num(w,":suspension-spring"),
            damping:num(w,":suspension-damping"),camber:num(w,":camber")*super::ANGLE,
        }};
        let mut sounds=HashMap::new();
        let snd=list(keyword(b,":sound"));
        for pair in snd.windows(2) { if let Some(key)=pair[0].symbol() {
            if let Some(Sexp::Atom(value))=list(Some(&pair[1])).get(1) {sounds.insert(key.trim_start_matches(':').to_owned(),value.trim_matches('"').to_owned());}
        }}
        let particles=list(keyword(b,":particles")); let mut points=HashMap::new();
        for key in [":thruster-local-pos",":exhaust-local-pos",":smoke-local-pos"] {
            let values=list(keyword(particles,key)).iter().filter_map(|x| match x {Sexp::List(v)=>Some(vec(v)),_=>None}).collect();
            points.insert(key[1..].to_owned(),values);
        }
        let seats=list(keyword(list(keyword(b,":rider")),":seat-array"));
        let seat=seats.iter().find_map(|v| {let s=list(Some(v));keyword(s,":position").map(|v|vec(list(Some(v))))}).unwrap_or(Vec3::ZERO);
        let gears=list(keyword(t,":gear-ratio-array")).iter().skip(5).filter_map(Sexp::number).take(num(t,":gear-count") as usize).collect();
        out.insert(name.to_owned(),Specs {
            mass:num(list(keyword(b,":info")),":mass"),hp:num(d,":hit-points"),toughness:num(d,":inv-toughness-factor"),
            gravity:num(list(keyword(b,":extra")),":gravity")/4096.,thrust:num(h,":max-engine-thrust")/4096.,
            response:num(h,":engine-response-rate"),speed:num(h,":max-xz-speed")/4096.,steer:num(h,":tire-steering-angle")*super::ANGLE,
            grip:num(h,":tire-static-friction"),sliding_grip:num(h,":tire-dynamic-friction"),jump:num(h,":jump-thrust-factor"),
            boost:num(h,":turbo-boost-factor"),boost_duration:num(h,":turbo-boost-duration"),
            height:num(list(keyword(b,":setup")),":settle-height")/4096.,seat,wheels:[wheel(":front-wheel"),wheel(":rear-wheel")],
            idle_rpm:num(e,":idle-rpm"),max_rpm:num(e,":max-rpm"),peak_rpm:num(e,":peak-torque-rpm"),
            audio_idle_rpm:num(snd,":idle-rpm"),audio_engine_rpm:num(snd,":engine-rpm"),idle_pitch_scale:num(snd,":idle-pitch-scale"),engine_pitch_scale:num(snd,":engine-pitch-scale"),idle_crossover:num(snd,":idle-crossover-rpm"),engine_crossover:num(snd,":engine-crossover-rpm"),
            upshift:num(t,":upshift-rpm"),downshift:num(t,":downshift-rpm"),final_drive:num(t,":final-drive-ratio"),gears,sounds,points,
        });
    }
    out
}
