//! Minecraft ladder columns, using Jak 3's own ladder animation channel and
//! TARGET-bank speeds (2.5 m/s up, 3 m/s down). No gravity while attached.
use glam::Vec3;
use crate::{Jak, State, CollideWorld, anim::{self, Anim, NumFunc}, math::*, pad::button};

#[derive(Clone, Copy, Debug)]
pub struct LadderSpan {
    pub anchor: Vec3,
    pub away: Vec3,
    pub bottom: f32,
    pub top: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase { Start, Idle, Up, Down, Exit, Jump }
#[derive(Clone, Copy, Debug)]
pub struct Ride {
    span: LadderSpan,
    phase: Phase,
    from: Vec3,
    to: Vec3,
    began: i64,
}
impl Jak {
    /// Keep the native 0.39 m ladder pose at the rung plane while the
    /// collision capsule stays outside Minecraft's thicker ladder shape.
    pub fn ladder_pose_weight(&self)->f32 {
        self.ladder.map_or(0.0,|r|match r.phase {
            Phase::Start=>((self.time-r.began) as f32/TICKS_PER_SECOND as f32/0.20).clamp(0.,1.),
            Phase::Exit|Phase::Jump=>1.-((self.time-r.began) as f32/TICKS_PER_SECOND as f32/0.20).clamp(0.,1.),
            _=>1.,
        })
    }

    fn ladder_anim(&mut self, phase: Phase, clip: Anim) {
        if let Some(ride)=self.ladder.as_mut(){ride.phase=phase;ride.began=self.time;}
        self.ja_push(seconds(0.10)); self.ja_play(clip,1.0);
    }
    fn leave_ladder(&mut self, state: State) {
        self.ladder=None;self.ladder_after=self.time+seconds(0.5);self.go(state);
    }
    fn ladder_landing(&mut self, span:LadderSpan, world:&mut dyn CollideWorld)->Option<Vec3>{
        for side in [-1.0,1.0] {
            // Outside ladders exit over their wall. Interior shafts can exit
            // back onto the roof deck through an open hatch instead.
            let at=Vec3::new(span.anchor.x,span.top,span.anchor.z)
                +span.away*(side*(self.control.body_radius+meters(0.65)));
            let Some(hit)=crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,at+Vec3::Y*meters(1.7),-Vec3::Y*meters(2.0),meters(0.02)) else{continue;};
            if hit.normal.y<0.9 || hit.intersect.y<span.top-meters(0.25) || hit.intersect.y>span.top+meters(1.55){continue;}
            let at=at.with_y(hit.intersect.y+meters(0.005));let r=self.control.body_radius*0.96;
            self.probe_cache.fill_box(world,at-Vec3::splat(r),at+Vec3::splat(self.control.standing_heights[2]+r));
            if self.control.standing_heights.iter().any(|h|self.probe_cache.overlaps_sphere(at+Vec3::Y*(*h+meters(0.01)),r)){continue;}
            return Some(at);
        }None
    }
    pub(crate) fn ladder_step(&mut self, world:&mut dyn CollideWorld)->bool {
        if self.state!=State::Ladder {self.ladder=None;}
        if self.ladder.is_none() {
            if self.time<self.ladder_after || self.state.is_board() || self.state.is_flut()
                || self.dark || self.lightjak.on || !matches!(self.state,State::Stance|State::Walk|State::Falling{..}|State::Jump{..}|State::DoubleJump{..}) {return false;}
            if self.pad.stick0_speed<0.3{return false;}
            let Some(span)=world.ladder(self.trans()) else{return false;};
            let intent=flatten(self.read_pad(),Vec3::Y).normalize_or_zero();
            if intent.dot(-span.away)<0.2{return false;}
            self.go(State::Ladder);
            self.ladder=Some(Ride{span,phase:Phase::Start,from:self.trans(),to:span.anchor,began:self.time});
            self.ladder_anim(Phase::Start,anim::LADDER_GET_ON);
        }
        let mut ride=self.ladder.unwrap();
        if !matches!(ride.phase,Phase::Exit|Phase::Jump) {
            let Some(span)=world.ladder(self.trans()) else{self.leave_ladder(State::Falling{uppercut:false});return false;};
            if flatten(span.anchor-ride.span.anchor,Vec3::Y).length()>meters(1.0){self.leave_ladder(State::Falling{uppercut:false});return false;}
            ride.span=span;
            if let Some(current)=self.ladder.as_mut(){current.span=span;}
        }
        self.control.transv=Vec3::ZERO;self.control.draw_offset_y=0.0;
        self.control.status &= !(crate::control::status::ON_SURFACE|crate::control::status::ON_GROUND|crate::control::status::TOUCH_SURFACE);
        self.trace.peak=self.control.trans.y;
        self.control.quat=forward_up_nopitch_quat(-ride.span.away,Vec3::Y);
        self.control.dir_targ=self.control.quat;self.gun.equipped=None;
        if self.pad.pressed(button::X) && !matches!(ride.phase,Phase::Exit|Phase::Jump) {
            self.ladder_anim(Phase::Jump,anim::LADDER_JUMP);return true;
        }
        match ride.phase {
            Phase::Start=>{
                let t=((self.time-ride.began) as f32/TICKS_PER_SECOND as f32/0.20).min(1.0);
                self.control.trans=self.control.trans.lerp(ride.span.anchor.with_y(self.trans().y),t);
                self.chan.eval(&self.anims);
                if self.chan.done(&self.anims){self.ladder_anim(Phase::Idle,anim::LADDER_IDLE);}
            }
            Phase::Jump=>{
                self.chan.eval(&self.anims);
                if self.chan.done(&self.anims){
                    let away=ride.span.away;
                    self.leave_ladder(State::Jump{min:crate::target::JUMP_HEIGHT_MAX,max:crate::target::JUMP_HEIGHT_MAX});
                    self.control.transv+=away*meters(4.0);return false;
                }
            }
            Phase::Exit=>{
                self.chan.eval(&self.anims);
                let frames=self.anims.info(anim::LADDER_GET_OFF).frames.saturating_sub(1).max(1) as f32;
                let t=(self.chan.frame/frames).clamp(0.0,1.0);
                let up=(t/0.65).min(1.0);let over=((t-0.65)/0.35).max(0.0);
                let at=ride.from.lerp(ride.to.with_x(ride.from.x).with_z(ride.from.z),up);
                self.control.trans=at.lerp(ride.to,over);
                if self.chan.done(&self.anims){
                    self.control.trans=ride.to;
                    self.control.status |= crate::control::status::ON_SURFACE | crate::control::status::ON_GROUND | crate::control::status::TOUCH_SURFACE;
                    self.control.last_time_on_surface=self.time;
                    self.control.ground_poly_normal=Vec3::Y;
                    self.control.gspot_pos=ride.to;
                    self.control.gspot_normal=Vec3::Y;
                    self.leave_ladder(State::Stance);return false;
                }
            }
            _=>{
                let stick=(128.0-f32::from(self.pad.lefty))/128.0;
                let phase=if stick>0.3{Phase::Up}else if stick < -0.3{Phase::Down}else{Phase::Idle};
                if phase!=ride.phase {self.ladder_anim(phase,match phase{Phase::Up=>anim::LADDER_UP,Phase::Down=>anim::LADDER_DOWN,_=>anim::LADDER_IDLE});}
                if phase==Phase::Idle {self.chan.eval_with(&self.anims,NumFunc::Loop{rate:1.0});}
                else {
                    let speed=if phase==Phase::Up{meters(2.5)}else{-meters(3.0)};
                    let mut y=(self.trans().y+speed*stick.abs().max(0.4)*SECONDS_PER_FRAME).min(ride.span.top);
                    let motion=Vec3::Y*(y-self.trans().y);
                    let trans=self.trans();
                    for h in self.control.standing_heights {
                        if crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,trans+Vec3::Y*h,motion,self.control.body_radius*0.96).is_some(){y=trans.y;break;}
                    }
                    self.control.trans=ride.span.anchor.with_y(y);
                    let clip=if phase==Phase::Up{anim::LADDER_UP}else{anim::LADDER_DOWN};
                    self.chan.frame=((y-ride.span.bottom)/meters(0.72)).rem_euclid(1.0)*self.anims.info(clip).frames.saturating_sub(1) as f32;
                    if y<ride.span.bottom {self.leave_ladder(State::Stance);return false;}
                    if phase==Phase::Up && y>=ride.span.top-meters(0.9) {
                        if let Some(to)=self.ladder_landing(ride.span,world){
                            if let Some(r)=self.ladder.as_mut(){r.from=self.control.trans;r.to=to;}
                            self.ladder_anim(Phase::Exit,anim::LADDER_GET_OFF);
                        }
                    }
                }
            }
        }
        true
    }
}


#[cfg(test)]mod terrain_regressions{
 use super::*;use crate::{Tri,Pat,PadInput,TriangleGrid};
 struct Ladder{available:bool,floor:TriangleGrid}
 impl Ladder {fn new()->Self{let v=[Vec3::new(-5.0*METER,0.0,-5.0*METER),Vec3::new(-5.0*METER,0.0,5.0*METER),Vec3::new(5.0*METER,0.0,5.0*METER),Vec3::new(5.0*METER,0.0,-5.0*METER)];Self{available:true,floor:TriangleGrid::new(vec![Tri{v:[v[0],v[1],v[2]],pat:Pat::default()},Tri{v:[v[0],v[2],v[3]],pat:Pat::default()}],METER)}}}
 impl CollideWorld for Ladder{
  fn fill(&mut self,a:Vec3,b:Vec3,out:&mut Vec<Tri>){self.floor.fill(a,b,out);}
  fn ladder(&mut self,p:Vec3)->Option<LadderSpan>{(self.available&&p.y<8.0*METER).then_some(LadderSpan{anchor:p.with_x(0.0).with_z(0.0),away:Vec3::NEG_Z,bottom:0.0,top:8.0*METER})}
 }
 #[test]fn climbing_uses_jak3_channels_stops_and_descends(){
  let mut w=Ladder::new();let mut j=Jak::new(Vec3::ZERO,0.0);let up=PadInput{left:[0.0,1.0],..Default::default()};
  for _ in 0..100{j.step(&up,&mut w,None);}
  assert_eq!(j.state,State::Ladder);assert_eq!(j.chan.anim,anim::LADDER_UP);let y=j.trans().y;assert!(y>1.0*METER);
  for _ in 0..30{j.step(&PadInput::default(),&mut w,None);}assert_eq!(j.chan.anim,anim::LADDER_IDLE);assert_eq!(j.trans().y,y);
  for _ in 0..25{j.step(&PadInput{left:[0.0,-1.0],..Default::default()},&mut w,None);}
  assert_eq!(j.chan.anim,anim::LADDER_DOWN);assert!(j.trans().y<y-0.9*METER);
 }
 #[test]fn removing_the_ladder_releases_jak_instead_of_leaving_him_suspended(){
  let mut w=Ladder::new();let mut j=Jak::new(Vec3::ZERO,0.0);
  for _ in 0..120{j.step(&PadInput{left:[0.0,1.0],..Default::default()},&mut w,None);}
  let y=j.trans().y;w.available=false;for _ in 0..10{j.step(&PadInput::default(),&mut w,None);}
  assert!(j.ladder.is_none());assert_ne!(j.state,State::Ladder);assert!(j.trans().y<y);
 }
 #[test]fn the_top_exit_lands_without_a_false_falling_frame(){
  let mut tris=Vec::new();
  for (y,z0) in [(0.0,-5.0),(8.0,0.9)] {
   let v=[Vec3::new(-5.0,y,z0),Vec3::new(-5.0,y,5.0),Vec3::new(5.0,y,5.0),Vec3::new(5.0,y,z0)].map(|v|v*METER);
   tris.push(Tri{v:[v[0],v[1],v[2]],pat:Pat::default()});tris.push(Tri{v:[v[0],v[2],v[3]],pat:Pat::default()});
  }
  let mut w=Ladder{available:true,floor:TriangleGrid::new(tris,METER)};
  let mut j=Jak::new(Vec3::ZERO,0.0);j.control.set_body_dimensions(0.432*METER,2.592*METER);
  let mut attached=false;let mut exited=false;
  for _ in 0..400 {
   j.step(&PadInput{left:[0.0,1.0],..Default::default()},&mut w,None);
   if j.state==State::Ladder{attached=true;}
   else if attached {assert!(matches!(j.state,State::Stance|State::Walk),"{:?}",j.state);assert!(j.control.on_surface());assert!(j.trans().y>=8.0*METER);exited=true;break;}
  }
  assert!(exited,"the ladder must exit onto the roof");
 }

}
