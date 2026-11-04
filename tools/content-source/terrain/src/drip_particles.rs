//! 26.3 dripstone particles: hanging, falling, landing/splash. Their world
//! clock and render interpolation are independent of Light Jak's entity clock.
use crate::{lighting::SkyLight,mesh::{Atlas,ChunkMesh,Vertex},pack::ResourceId,scene::Scene};
use glam::{DVec3,Vec3};
use minecraftoss_core::block::FluidKind;
use minecraftoss_world::level::dripstone::DripEffect;
use minecraftoss_player::{Block,collision_shape_boxes,rng::LegacyRandom};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Stage {Hang,Fall,Land,Splash}
#[derive(Clone,Debug)]
struct Drop {at:DVec3,previous:DVec3,velocity:DVec3,fluid:FluidKind,stage:Stage,life:i32,size:f32,splash:u8}
impl Drop {
    fn new(at:DVec3,fluid:FluidKind,stage:Stage,r:&mut LegacyRandom)->Self {
        let size=0.1*(r.next_float()*0.5+0.5)*2.0;
        let life=match stage {Stage::Hang=>40,Stage::Fall=>(64.0/(r.next_double()*0.8+0.2)) as i32,Stage::Land=>(16.0/(r.next_double()*0.8+0.2)) as i32,Stage::Splash=>(8.0/(r.next_double()*0.8+0.2)) as i32};
        let velocity=if stage==Stage::Splash {DVec3::Y*f64::from(r.next_float()*0.2+0.1)}else{DVec3::ZERO};
        Self{at,previous:at,velocity,fluid,stage,life,size,splash:(r.next_float()*4.0) as u8}
    }
    fn center(&self,partial:f32)->DVec3 {self.previous.lerp(self.at,f64::from(partial.clamp(0.0,1.0)))}
    fn sprite(&self)->String {match self.stage {Stage::Hang=>"drip_hang".into(),Stage::Fall=>"drip_fall".into(),Stage::Land=>"drip_land".into(),Stage::Splash=>format!("splash_{}",self.splash.min(3))}}
    fn color(&self)->[f32;3] {
        if self.stage==Stage::Splash {return [1.0;3];}
        if self.fluid==FluidKind::Water {return [0.2,0.3,1.0];}
        if self.stage==Stage::Hang {let age=(40-self.life) as f32;[1.0,16.0/(age+16.0),4.0/(age+8.0)]}else{[1.0,0.2857143,0.083333336]}
    }
}
/// Cosmetic events are bounded; block simulation and scheduled fill are server-owned.
pub struct DripParticles {particles:Vec<Drop>,random:LegacyRandom}
impl Default for DripParticles {
    fn default()->Self {Self{particles:Vec::new(),random:LegacyRandom::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0,|d|d.as_nanos() as u64))}}
}
impl DripParticles {
    pub fn event(&mut self,event:DripEffect,sounds:&mut Vec<(String,DVec3,f32,f32)>) {
        let (tip,fluid)=match event {
            DripEffect::Drop{tip,fluid}=>(tip,fluid),
            DripEffect::DryMud{tip,..}=>(tip,FluidKind::Water),
            DripEffect::Fill{at,fluid}=>{sounds.push((format!("minecraft:block.pointed_dripstone.drip_{}_into_cauldron",if fluid==FluidKind::Water{"water"}else{"lava"}),DVec3::new(at.x as f64+0.5,at.y as f64+0.5,at.z as f64+0.5),1.0,1.0));return;}
        };
        // The host's block mesh has no random block offset. Match its tip;
        // SHAPE_TIP_DOWN.minY (5/16) minus 1/16, as spawnDripParticle.
        if self.particles.len()<512 {self.particles.push(Drop::new(DVec3::new(tip.x as f64+0.5,tip.y as f64+0.25,tip.z as f64+0.5),fluid,Stage::Hang,&mut self.random));}
    }
    pub fn tick<S:Scene>(&mut self,scene:&S,sounds:&mut Vec<(String,DVec3,f32,f32)>) {
        let r=&mut self.random;let mut children=Vec::new();
        self.particles.retain_mut(|p| {
            p.previous=p.at;let old=p.life;p.life-=1;
            if old<=0 {if p.stage==Stage::Hang {children.push(Drop::new(p.at,p.fluid,Stage::Fall,r));}return false;}
            p.velocity.y-=f64::from(match p.stage {Stage::Hang=>0.06_f32*0.02,Stage::Splash=>0.04, _=>0.06});
            let requested=p.velocity;let mut moved=requested;
            let mut min=[p.at.x-0.005,p.at.y,p.at.z-0.005];let mut max=[p.at.x+0.005,p.at.y+0.01,p.at.z+0.005];
            let end=p.at+requested;let mut boxes=Vec::new();
            for x in (p.at.x.min(end.x)-0.01).floor() as i32..=(p.at.x.max(end.x)+0.01).floor() as i32 {
                for y in p.at.y.min(end.y).floor() as i32..=(p.at.y.max(end.y)+0.01).floor() as i32 {
                    for z in (p.at.z.min(end.z)-0.01).floor() as i32..=(p.at.z.max(end.z)+0.01).floor() as i32 {
                        if let Some(b)=scene.block((x,y,z)) {let shape=Block{id:b.id.key(),properties:b.properties.clone()};for (mut a,mut b) in collision_shape_boxes(&shape){for (axis,v) in [x,y,z].into_iter().enumerate(){a[axis]+=v as f64;b[axis]+=v as f64;}boxes.push((a,b));}}
                    }
                }
            }
            for axis in [1,0,2] {let (a,b)=((axis+1)%3,(axis+2)%3);for (near,far) in &boxes {
                if min[a]>=far[a]||max[a]<=near[a]||min[b]>=far[b]||max[b]<=near[b]{continue;}
                if moved[axis]>0.0&&max[axis]<=near[axis]{moved[axis]=moved[axis].min(near[axis]-max[axis]);}
                else if moved[axis]<0.0&&min[axis]>=far[axis]{moved[axis]=moved[axis].max(far[axis]-min[axis]);}
            }min[axis]+=moved[axis];max[axis]+=moved[axis];}
            p.at+=moved;
            let ground=requested.y<0.0&&requested.y!=moved.y;
            if ground&&p.stage==Stage::Fall {
                children.push(Drop::new(p.at,p.fluid,if p.fluid==FluidKind::Water{Stage::Splash}else{Stage::Land},r));
                sounds.push((format!("minecraft:block.pointed_dripstone.drip_{}",if p.fluid==FluidKind::Water{"water"}else{"lava"}),p.at,0.3+r.next_float()*0.7,1.0));return false;
            }
            if p.stage==Stage::Hang {p.velocity*=0.02;}
            p.velocity*=f64::from(0.98_f32);
            if ground&&p.stage==Stage::Splash {if r.next_float()<0.5{return false;}p.velocity.x*=0.7;p.velocity.z*=0.7;}
            let cell=(p.at.x.floor() as i32,p.at.y.floor() as i32,p.at.z.floor() as i32);
            if let Some(f)=scene.fluid_at(cell) {let same=matches!((p.fluid,f.kind),(FluidKind::Water,crate::fluid::FluidKind::Water)|(FluidKind::Lava,crate::fluid::FluidKind::Lava));if (same||p.stage==Stage::Splash)&&p.at.y<(cell.1 as f64+f.amount as f64/9.0){return false;}}
            true
        });
        let available=512-self.particles.len();self.particles.extend(children.into_iter().take(available));
    }
    pub fn append_mesh(&self,mesh:&mut ChunkMesh,atlas:&Atlas,forward:Vec3,partial:f32,light:&SkyLight) {
        let right=forward.cross(Vec3::Y).normalize_or_zero();let up=right.cross(forward).normalize_or_zero();
        for p in &self.particles {let Ok(id)=ResourceId::parse(&format!("minecraft:particle/{}",p.sprite()))else{continue;};if !atlas.contains(&id){continue;}
            let [u0,v0,u1,v1]=atlas.region(&id);let at=p.center(partial);let cell=(at.x.floor() as i32,at.y.floor() as i32,at.z.floor() as i32);let color=p.color();let start=mesh.vertices.len() as u32;
            for (x,y,uv) in [(-1.0,-1.0,[u0,v1]),(-1.0,1.0,[u0,v0]),(1.0,1.0,[u1,v0]),(1.0,-1.0,[u1,v1])] {let pos=at.as_vec3()+(right*x+up*y)*p.size;mesh.vertices.push(Vertex{position:pos.to_array(),uv,color:[color[0],color[1],color[2],1.0],sky_light:light.get(cell) as f32,block_light:light.get_block(cell) as f32});}
            mesh.indices.extend([start,start+2,start+1,start,start+3,start+2]);mesh.faces+=1;
        }
    }
}
#[cfg(test)]mod tests {
    use super::*;use crate::scene::{HandcraftedScene,Block as SceneBlock};
    #[test]fn dripstone_hangs_then_falls_lands_on_shape_and_expires(){
        let mut p=DripParticles{particles:Vec::new(),random:LegacyRandom::new(42)};let mut sounds=Vec::new();let mut scene=HandcraftedScene::default();scene.set((0,0,0),Some(SceneBlock::new("minecraft:stone_slab").with("type","bottom")));
        p.event(DripEffect::Drop{tip:minecraftoss_core::BlockPos::new(0,3,0),fluid:FluidKind::Water},&mut sounds);
        for _ in 0..40 {p.tick(&scene,&mut sounds);}assert_eq!(p.particles[0].stage,Stage::Hang);p.tick(&scene,&mut sounds);assert_eq!(p.particles[0].stage,Stage::Fall);
        p.tick(&scene,&mut sounds);let d=&p.particles[0];assert!(d.previous.y>d.at.y);assert!((d.center(0.5).y-(d.previous.y+d.at.y)*0.5).abs()<1e-10);
        for _ in 0..30{p.tick(&scene,&mut sounds);}assert_eq!(sounds.len(),1);assert!((sounds[0].1.y-0.5).abs()<1e-6,"drop lands on actual half slab, not full cube");
        for _ in 0..400{p.tick(&scene,&mut sounds);}assert!(p.particles.is_empty());
    }
    #[test]fn dripstone_events_bound_particles_and_use_native_fill_sound(){let mut p=DripParticles::default();let mut s=Vec::new();let at=minecraftoss_core::BlockPos::new(0,3,0);for _ in 0..2048{p.event(DripEffect::Drop{tip:at,fluid:FluidKind::Lava},&mut s);}assert_eq!(p.particles.len(),512);p.event(DripEffect::Fill{at,fluid:FluidKind::Lava},&mut s);assert_eq!(s[0].0,"minecraft:block.pointed_dripstone.drip_lava_into_cauldron");}
}
