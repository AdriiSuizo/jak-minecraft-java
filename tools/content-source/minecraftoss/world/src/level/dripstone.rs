//! Java26.3 PointedDripstoneBlock fluid transfer and AbstractCauldronBlock's
//! delayed receipt. Runs with the existing level random/scheduler, not a host scan.
use super::Level;
use minecraftoss_core::{BlockPos,BlockStateId};
use minecraftoss_core::block::{FluidKind,flags};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum DripEffect {Drop{tip:BlockPos,fluid:FluidKind},Fill{at:BlockPos,fluid:FluidKind},DryMud{at:BlockPos,tip:BlockPos}}
impl Level<'_> {
    fn pointed_down(&self,state:BlockStateId)->bool {
        self.name(state)=="minecraft:pointed_dripstone"&&self.registries().blocks.property(state,"vertical_direction")==Some("down")
    }
    fn drip_tip(&self,state:BlockStateId)->bool {
        (self.is_a(state,"SpeleothemBlock")||self.name(state)=="minecraft:pointed_dripstone")&&self.registries().blocks.property(state,"thickness")==Some("tip")
    }
    fn free_drip_tip(&self,state:BlockStateId)->bool {
        self.pointed_down(state)&&self.drip_tip(state)&&self.registries().blocks.property(state,"waterlogged")!=Some("true")
    }
    fn drip_fluid(&self,tip:BlockPos)->Option<(BlockPos,BlockStateId,FluidKind)> {
        if !self.pointed_down(self.block(tip)){return None;}
        for step in 1..11 {
            let at=BlockPos::new(tip.x,tip.y+step,tip.z);let state=self.block(at);
            if self.name(state)!="minecraft:pointed_dripstone" {
                let source=BlockPos::new(at.x,at.y+1,at.z);let state=self.block(source);
                if self.name(state)=="minecraft:mud"&&!self.boolean_attribute_at("gameplay/water_evaporates",source){return Some((source,state,FluidKind::Water));}
                let fluid=self.registries().blocks.state(state).fluid?;
                // Java compares with Fluids.WATER/LAVA, not FLOWING_WATER/LAVA.
                return fluid.source.then_some((source,state,fluid.kind));
            }
            if !self.pointed_down(state){return None;}
        }
        None
    }
    fn drip_through(&self,_at:BlockPos,state:BlockStateId)->bool {
        let blocks=&self.registries().blocks;
        if blocks.is_air(state){return true;}
        if blocks.state(state).flags&flags::SOLID_RENDER!=0||blocks.state(state).fluid.is_some(){return false;}
        // REQUIRED_SPACE_TO_DRIP_THROUGH_NON_SOLID_BLOCK is the central
        // four-pixel column, so a thin open gate can leave a real route.
        !blocks.collision_boxes(state).iter().any(|b|b[0]<10.0/16.0&&b[3]>6.0/16.0&&b[2]<10.0/16.0&&b[5]>6.0/16.0&&b[1]<1.0&&b[4]>0.0)
    }
    fn accepts_drip(&self,state:BlockStateId,fluid:FluidKind)->bool {
        self.name(state)=="minecraft:cauldron"||(self.name(state)=="minecraft:water_cauldron"&&fluid==FluidKind::Water)
    }
    /// Native randomTick feeds this its one nextFloat. Public for the same
    /// direct block-event entry point exposed by PointedDripstoneBlock.
    pub fn transfer_dripstone_fluid(&mut self,at:BlockPos,chance:f32) {
        let state=self.block(at);
        static TRACE:std::sync::OnceLock<bool>=std::sync::OnceLock::new();
        if *TRACE.get_or_init(||std::env::var_os("SIM_RANDOM_TICK_LOG").is_some()) {
            eprintln!("DRIP_TRACE tick={} at={at:?} chance={chance} down={} tip={} thickness={:?} classes={:?} above={} fluid={:?} below={:?}",self.game_time,self.pointed_down(state),self.drip_tip(state),self.registries().blocks.property(state,"thickness"),self.registries().blocks.block(self.block_id(state)).classes(),self.name(self.block(at.above())),self.drip_fluid(at),(1..=4).map(|d|self.name(self.block(BlockPos::new(at.x,at.y-d,at.z)))).collect::<Vec<_>>());
        }
        if chance>=0.17578125||!self.pointed_down(state)||self.block_id(self.block(at.above()))==self.block_id(state){return;}
        let Some((source,source_state,fluid))=self.drip_fluid(at)else{return;};
        if chance>=if fluid==FluidKind::Water{0.17578125}else{0.05859375}{return;}
        let mut tip=None;
        for distance in 0..11 {
            let pos=BlockPos::new(at.x,at.y-distance,at.z);let here=self.block(pos);
            if self.drip_tip(here){tip=Some(pos);break;}
            if self.block_id(here)!=self.block_id(state)||!self.pointed_down(here){return;}
        }
        let Some(tip)=tip else{return;};
        if self.name(source_state)=="minecraft:mud" {
            let clay=self.registries().blocks.parse_state("minecraft:clay").expect("native clay");
            // Mud's top is 14/16; the new clay top is 16/16. Lift only
            // entities intersecting the newly occupied volume (pushEntitiesUp).
            let added=super::physics::Aabb::new(source.x as f64,source.y as f64+0.875,source.z as f64,source.x as f64+1.0,source.y as f64+1.0,source.z as f64+1.0);
            for e in &mut self.entities {if !e.removed&&e.bb.intersects(&added){let dy=source.y as f64+1.0-e.bb.min[1];e.pos[1]+=dy;e.bb=e.bb.moved([0.0,dy,0.0]);}}
            self.set_block_and_update(source,clay);self.drip_effects.push(DripEffect::DryMud{at:source,tip});return;
        }
        for distance in 1..11 {
            let pos=BlockPos::new(tip.x,tip.y-distance,tip.z);let here=self.block(pos);
            if self.accepts_drip(here,fluid){
                self.drip_effects.push(DripEffect::Drop{tip,fluid});
                self.schedule_block_tick(pos,here,50+distance);
                if *TRACE.get().unwrap_or(&false){eprintln!("DRIP_SCHEDULE now={} target={pos:?} due={} classes={:?}",self.game_time,self.game_time+50+i64::from(distance),self.registries().blocks.block(self.block_id(here)).classes());}return;
            }
            if !self.drip_through(pos,here){return;}
        }
    }
    pub(super) fn receive_cauldron_drip(&mut self,at:BlockPos,state:BlockStateId) {
        if std::env::var_os("SIM_RANDOM_TICK_LOG").is_some(){eprintln!("DRIP_RECEIVE now={} target={at:?} state={}",self.game_time,self.name(state));}
        let mut tip=None;
        for distance in 1..11 {
            let pos=BlockPos::new(at.x,at.y+distance,at.z);let here=self.block(pos);
            if self.free_drip_tip(here){tip=Some(pos);break;}
            if !self.drip_through(pos,here){return;}
        }
        let Some((_,_,fluid))=tip.and_then(|tip|self.drip_fluid(tip))else{return;};
        if !self.accepts_drip(state,fluid){return;}
        let blocks=&self.registries().blocks;
        let next=if self.name(state)=="minecraft:cauldron" {
            blocks.parse_state(if fluid==FluidKind::Water{"minecraft:water_cauldron"}else{"minecraft:lava_cauldron"}).expect("native cauldron")
        }else{
            let level=blocks.property(state,"level").and_then(|n|n.parse::<i32>().ok()).unwrap_or(1);
            if level>=3{return;}
            blocks.with_property(state,"level",&(level+1).to_string()).expect("native layered cauldron")
        };
        self.set_block_and_update(at,next);self.drip_effects.push(DripEffect::Fill{at,fluid});
    }
}
