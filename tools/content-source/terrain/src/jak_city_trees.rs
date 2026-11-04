//! Natural trees reserve a whole root/crown before FEATURES. Cutting a tall
//! jungle tree by urban columns leaves its remote branches suspended; avoid
//! creating that tree instead. This host adaptation never filters saplings in
//! a live level and never changes feature placement outside the settlement.
use crate::jak_structures::{Placed,Style};
use minecraftoss_generator::feature::Region;
use minecraftoss_world::chunk_map::WorldGen;
/// One chunk of clearance covers the native mega-jungle branch offset
/// (up to five), its foliage, and clear_city_trees' six-block leaf halo.
const CROWN_MARGIN:i32=16;
pub(crate) struct RootArea {min:(i32,i32),size:(usize,usize),prefix:Vec<u32>}
impl RootArea {
    pub(crate) fn from_blocks(blocks:&[Placed])->Self {
        let mut lo=(i32::MAX,i32::MAX);let mut hi=(i32::MIN,i32::MIN);
        for &((x,_,z),s)in blocks {if s=="minecraft:air"{lo=(lo.0.min(x),lo.1.min(z));hi=(hi.0.max(x),hi.1.max(z));}}
        if lo.0>hi.0{return Self{min:(0,0),size:(0,0),prefix:Vec::new()};}
        let(w,h)=((hi.0-lo.0+1)as usize,(hi.1-lo.1+1)as usize);let stride=w+1;let mut prefix=vec![0;(w+1)*(h+1)];
        for &((x,_,z),s)in blocks {if s=="minecraft:air"{prefix[(z-lo.1+1)as usize*stride+(x-lo.0+1)as usize]=1;}}
        for z in 1..=h{for x in 1..=w {let i=z*stride+x;prefix[i]+=prefix[i-1]+prefix[i-stride]-prefix[i-stride-1];}}
        Self{min:lo,size:(w,h),prefix}
    }
    pub(crate) fn bytes(&self)->usize{self.prefix.capacity()*std::mem::size_of::<u32>()}
    fn excludes(&self,x:i32,z:i32)->bool {
        let(w,h)=self.size;if w==0{return false;}
        let left=(x-CROWN_MARGIN-self.min.0).clamp(0,w as i32)as usize;
        let top=(z-CROWN_MARGIN-self.min.1).clamp(0,h as i32)as usize;
        let right=(x+CROWN_MARGIN-self.min.0+1).clamp(0,w as i32)as usize;
        let bottom=(z+CROWN_MARGIN-self.min.1+1).clamp(0,h as i32)as usize;
        if left>=right||top>=bottom{return false;}let s=w+1;
        self.prefix[bottom*s+right]+self.prefix[top*s+left]>self.prefix[top*s+right]+self.prefix[bottom*s+left]
    }
}
pub(crate) fn prepare(region:&mut Region,gen:&WorldGen){
    if !gen.terrain.host_structures||!crate::jak_structures::natural_jak_world(gen){return;}
    let seed=gen.terrain.seed;let pos=region.center();let(x0,z0)=(pos.x*16-16,pos.z*16-16);let reach=crate::jak_town::REACH+CROWN_MARGIN;
    let mut roots=std::collections::HashSet::new();
    for cx in (x0-reach).div_euclid(320)..=(x0+47+reach).div_euclid(320){
        for cz in (z0-reach).div_euclid(320)..=(z0+47+reach).div_euclid(320){
            let Some((vx,vz))=crate::jak_structures::village_spot(seed,(cx,cz))else{continue;};
            if vx<x0-reach||vx>x0+47+reach||vz<z0-reach||vz>z0+47+reach||crate::wasteland::is_wasteland(gen,vx,vz){continue;}
            let style=if crate::jak_lands::is_in(&crate::snowy::LAND,gen,vx,vz){Style::Sandover("jak:teja_nevada","jak:cumbrera_nevada")}
                else if crate::jak_lands::is_in(&crate::swamp::LAND,gen,vx,vz){Style::Sandover("jak:teja_pantano","jak:cumbrera_pantano")}else{Style::Slums};
            let Some(plan)=crate::jak_structures::settlement_plan(seed,(cx,cz),gen,false,style)else{continue;};
            let area=plan.trees.get_or_init(||RootArea::from_blocks(&plan.blocks));
            for z in z0..z0+48{for x in x0..x0+48{if area.excludes(x,z){roots.insert((x,z));}}}
        }
    }
    region.exclude_tree_roots(roots);
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn whole_tree_reservation_spans_chunk_edges_but_preserves_distant_forest(){
        let area=RootArea::from_blocks(&[((-16,70,-17),"minecraft:air"),((-2,70,-17),"minecraft:air"),((-10,70,4),"minecraft:stone")]);
        assert!(area.excludes(-32,-17));assert!(area.excludes(14,-17));assert!(area.excludes(-16,-33));
        assert!(!area.excludes(-33,-17));assert!(!area.excludes(15,-17));assert!(!area.excludes(-10,4),"authored solids alone are not a tree clearing");
        assert!(!RootArea::from_blocks(&[]).excludes(0,0));
    }
    #[test]fn prefix_footprint_respects_irregular_holes_and_multiple_air_heights(){
        let blocks=vec![((0,70,0),"minecraft:air"),((0,71,0),"minecraft:air"),((50,70,50),"minecraft:air")];
        let area=RootArea::from_blocks(&blocks);assert!(area.excludes(0,0));assert!(area.excludes(50,50));assert!(!area.excludes(25,25));
    }
}

#[cfg(test)]mod native_tree_tests {
    #[test]fn a_reserved_root_rejects_the_whole_native_oak_before_any_branches_are_written(){
        use std::sync::Arc;
        use minecraftoss_core::{Chunk,ChunkPos,BlockPos,Registries,registries::DataPaths,random::WorldgenRandom};
        use minecraftoss_generator::{terrain::TerrainGenerator,feature::{Region,Ctx,place_feature}};
        let r=Arc::new(Registries::load(&DataPaths::discover().unwrap()).unwrap());
        let gen=minecraftoss_world::chunk_map::WorldGen::new(Arc::new(TerrainGenerator::overworld(r.clone(),20261101).unwrap())).unwrap();
        let biome=r.biomes.id("minecraft:plains").unwrap();let grass=r.blocks.parse_state("minecraft:grass_block[snowy=false]").unwrap();
        let chunks=||(-1..=1).flat_map(|z|(-1..=1).map(move|x|(x,z))).map(|(x,z)|{
            let mut c=Chunk::new(ChunkPos::new(x,z),-64,384,biome);
            for z in 0..16{for x in 0..16{c.set_block_raw(x,70,z,grass,&r);}}c
        }).collect();
        let id=gen.library.feature_by_name("minecraft:oak").unwrap();let origin=BlockPos::new(8,71,8);
        let mut ordinary=Region::new(ChunkPos::new(0,0),chunks(),r.clone(),20261101);
        assert!(place_feature(&mut Ctx{lib:&gen.library,region:&mut ordinary},&mut WorldgenRandom::new(99),id,origin),"the original native feature must place on this ground");
        assert_ne!(ordinary.block(8,71,8),minecraftoss_core::BlockStateId::AIR);
        let mut reserved=Region::new(ChunkPos::new(0,0),chunks(),r,20261101);reserved.exclude_tree_roots([(8,8)]);
        assert!(!place_feature(&mut Ctx{lib:&gen.library,region:&mut reserved},&mut WorldgenRandom::new(99),id,origin));
        for y in 71..96{for x in 0..16{for z in 0..16{assert_eq!(reserved.block(x,y,z),minecraftoss_core::BlockStateId::AIR);}}}
    }
}

#[cfg(test)]mod dimension_tests {
    #[test]fn native_city_reservation_applies_before_features_only_in_its_natural_overworld(){
        use std::sync::Arc;use minecraftoss_core::{Chunk,ChunkPos,BlockPos,Registries,registries::DataPaths};
        use minecraftoss_generator::{terrain::TerrainGenerator,feature::Region};
        let r=Arc::new(Registries::load(&DataPaths::discover().unwrap()).unwrap());
        let mut gen=minecraftoss_world::chunk_map::WorldGen::new(Arc::new(TerrainGenerator::overworld(r.clone(),20261101).unwrap())).unwrap();
        Arc::get_mut(&mut gen.terrain).unwrap().host_structures=true;
        let center=ChunkPos::new(20010>>4,14814>>4);let biome=r.biomes.id("minecraft:jungle").unwrap();
        let fresh=||Region::new(center,(-1..=1).flat_map(|z|(-1..=1).map(move|x|(x,z))).map(|(x,z)|Chunk::new(ChunkPos::new(center.x+x,center.z+z),-64,384,biome)).collect(),r.clone(),20261101);
        let root=BlockPos::new(20010,72,14814);let mut natural=fresh();super::prepare(&mut natural,&gen);assert!(!natural.allows_tree(root));
        gen.dimension_type="minecraft:the_nether".into();let mut nether=fresh();super::prepare(&mut nether,&gen);assert!(nether.allows_tree(root));
        gen.dimension_type="minecraft:overworld".into();Arc::get_mut(&mut gen.terrain).unwrap().void=true;
        let mut exhibit=fresh();super::prepare(&mut exhibit,&gen);assert!(exhibit.allows_tree(root));
    }
}
