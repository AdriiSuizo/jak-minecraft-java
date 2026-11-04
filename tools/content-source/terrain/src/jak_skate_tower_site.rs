//! Physical palace placement shares the city's reserved urban core.
//! Its complete base, approaches and neighbouring houses are planned together.
use crate::jak_city_arena as tower;
use crate::jak_structures::{Land, Placed, Style};
use minecraftoss_core::{block::BlockStateId, chunk::Chunk};
use minecraftoss_world::chunk_map::WorldGen;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

const CELL: i32 = 320;
// Generation lookup must cover the complete model and its skirt road,
// not just the tower's centre relative to the owning city.
const REACH: i32 = 512;
const CITY_LINK: i32 = 384;
// Matches the canopy halo consumed by jak_structures::clear_city_trees.
const TREE_HALO: i32 = 6;
type Pos = (i32, i32, i32);
#[derive(Clone, Debug)]
pub struct Site {
    pub city: (i32, i32),
    pub center: Pos,
    /// A four-block street; heights use the same eighths as the city.
    road: BTreeMap<(i32, i32), i32>,
}
impl Site {
    fn contains_chunk(&self, x: i32, z: i32) -> bool {
        (x + 16 + TREE_HALO > self.center.0 + tower::FOOTPRINT_MIN[0]
            && x - TREE_HALO < self.center.0 + tower::FOOTPRINT_MAX[0]
            && z + 16 + TREE_HALO > self.center.2 + tower::FOOTPRINT_MIN[1]
            && z - TREE_HALO < self.center.2 + tower::FOOTPRINT_MAX[1])
            || self
                .road
                .keys()
                .any(|&(rx, rz)| (x - TREE_HALO..x + 16 + TREE_HALO).contains(&rx)
                    && (z - TREE_HALO..z + 16 + TREE_HALO).contains(&rz))
    }
    /// Inclusive chunk bounds of every native/road write and canopy cleanup.
    pub fn affected_chunk_bounds(&self)->(i32,i32,i32,i32) {
        let(mut x0,mut z0)=(self.center.0+tower::FOOTPRINT_MIN[0],self.center.2+tower::FOOTPRINT_MIN[1]);
        let(mut x1,mut z1)=(self.center.0+tower::FOOTPRINT_MAX[0]-1,self.center.2+tower::FOOTPRINT_MAX[1]-1);
        for &(x,z) in self.road.keys(){x0=x0.min(x);x1=x1.max(x);z0=z0.min(z);z1=z1.max(z);}
        ((x0-TREE_HALO).div_euclid(16),(z0-TREE_HALO).div_euclid(16),
            (x1+TREE_HALO).div_euclid(16),(z1+TREE_HALO).div_euclid(16))
    }
    pub fn entrance(&self) -> [f64; 3] {
        [
            self.center.0 as f64 + tower::ACCESS[0],
            self.center.1 as f64 + tower::ACCESS[1],
            self.center.2 as f64 + tower::ACCESS[2],
        ]
    }
}

// A rectangle belongs to the nearest potential city. These four affine
// distance comparisons prevent towers from overlapping one another without
// recursive planning of neighbouring towers or generation-order dependence.
fn reserved_for(seed: i64, city: (i32, i32), lo: (i32, i32), hi: (i32, i32)) -> bool {
    for cx in (lo.0 - REACH).div_euclid(CELL)..=(hi.0 + REACH).div_euclid(CELL) {
        for cz in (lo.1 - REACH).div_euclid(CELL)..=(hi.1 + REACH).div_euclid(CELL) {
            let Some(other) = crate::jak_structures::village_spot(seed, (cx, cz)) else {
                continue;
            };
            if other == city {
                continue;
            }
            let r = crate::jak_town::REACH + 4;
            if lo.0 <= other.0 + r
                && hi.0 >= other.0 - r
                && lo.1 <= other.1 + r
                && hi.1 >= other.1 - r
            {
                return false;
            }
            for p in [lo, (lo.0, hi.1), (hi.0, lo.1), hi] {
                let d = |c: (i32, i32)| {
                    (p.0 as i64 - c.0 as i64).pow(2) + (p.1 as i64 - c.1 as i64).pow(2)
                };
                if d(city) >= d(other) {
                    return false;
                }
            }
        }
    }
    true
}

fn parcel_bounds(city:(i32,i32),dz:i32)->((i32,i32),(i32,i32)) {
    let (x,z)=(city.0-208,city.1+dz);
    ((x+tower::FOOTPRINT_MIN[0],z+tower::FOOTPRINT_MIN[1]),
        (x+tower::FOOTPRINT_MAX[0]-1,z+tower::FOOTPRINT_MAX[1]-1))
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct PlanStats {
    pub candidates: usize,
    pub reserved: usize,
    pub city_biome: usize,
    pub wet: usize,
    pub elevation: usize,
    pub parcel_biome: usize,
    pub parcels_viable: usize,
    pub city_missing: usize,
    pub road_rejected: usize,
    pub accepted: usize,
    pub visited_rejected: usize,
}
impl PlanStats {
    fn add(&mut self, other:&Self) {
        self.candidates+=other.candidates; self.reserved+=other.reserved;
        self.city_biome+=other.city_biome; self.wet+=other.wet;
        self.elevation+=other.elevation; self.parcel_biome+=other.parcel_biome;
        self.parcels_viable+=other.parcels_viable; self.city_missing+=other.city_missing;
        self.road_rejected+=other.road_rejected; self.accepted+=other.accepted;
        self.visited_rejected+=other.visited_rejected;
    }
}

// The crown may overhang water. Ground must support the octagonal base
// and its entrance; biome exclusivity still covers the entire native model.
fn ground_footprint(x:i32,z:i32)->bool {
    tower::support_floor(x,z).is_some()||((43..67).contains(&x)&&(-12..-8).contains(&z))
}
const MAX_BASE_Y:i32=320-tower::HEIGHT;
const MAX_GROUND_RELIEF:i32=24;
fn parcel_center(seed:i64,city:(i32,i32),dz:i32,land:&dyn Land,stats:&mut PlanStats)->Option<Pos> {
    let (lo,hi)=parcel_bounds(city,dz);
    if !reserved_for(seed,city,lo,hi) {stats.reserved+=1;return None;}
    let (cx,cz)=(city.0-208,city.1+dz);
    let mut low=i32::MAX;let mut high=i32::MIN;
    for wx in (lo.0..=hi.0).step_by(16) {for wz in (lo.1..=hi.1).step_by(16) {
        let(h,wet)=land.probe_column(wx,wz);
        if ground_footprint(wx-cx,wz-cz) {
            low=low.min(h);high=high.max(h);
            if wet {stats.wet+=1;return None;}
            if h>MAX_BASE_Y || high-low>MAX_GROUND_RELIEF {stats.elevation+=1;return None;}
        }
        if !land.probe_city_allowed(wx,wz,h,false) {stats.parcel_biome+=1;return None;}
    }}
    // Full exact validation remains mandatory after the sparse early rejects.
    for wx in lo.0..=hi.0 {for wz in lo.1..=hi.1 {
        if !land.city_allowed(wx,wz,false) {stats.parcel_biome+=1;return None;}
        if ground_footprint(wx-cx,wz-cz) {
            if land.wet(wx,wz) {stats.wet+=1;return None;}
            let h=land.ground(wx,wz);low=low.min(h);high=high.max(h);
            if high>MAX_BASE_Y || high-low>MAX_GROUND_RELIEF {stats.elevation+=1;return None;}
        }
    }}
    stats.parcels_viable+=1;
    Some((cx,high,cz))
}

fn connect_road(city:(i32,i32),center:Pos,land:&dyn Land,blocks:&[Placed])->Option<Site> {
    let (x,high,z)=center;
    let end = (city.0 - 56, city.1 - 28);
    let mut end_height = None;
    // The actual street at the attachment, including generated partial slabs.
    for &(p, s) in blocks {
        if (p.0, p.2) == end {
            if s == "jak:barrio_losa" {
                end_height = Some((p.1 + 1) * 8);
            }
            if let Some(n) = s
                .strip_prefix("jak:barrio_losa_capa[layers=")
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse::<i32>().ok())
            {
                end_height = Some(p.1 * 8 + n);
            }
        }
    }
    let end_height = end_height?;
    let corners = [
        (x + 67, z - 10),
        (city.0 - 80, z - 10),
        (city.0 - 80, end.1),
        end,
    ];
    let mut path = Vec::new();
    for pair in corners.windows(2) {
        let (mut a, b) = (pair[0], pair[1]);
        while a != b {
            path.push(a);
            a.0 += (b.0 - a.0).signum();
            a.1 += (b.1 - a.1).signum();
        }
    }
    path.push(end);
    if (end_height - high * 8).abs() > 2 * (path.len() as i32 - 1) {
        return None;
    }
    let mut road = BTreeMap::new();
    for (i, &(wx, wz)) in path.iter().enumerate() {
        let h = high * 8
            + ((end_height - high * 8) as f64 * i as f64 / (path.len() - 1).max(1) as f64).round()
                as i32;
        for dx in -2..=1 {
            for dz in -2..=1 {
                let at = (wx + dx, wz + dz);
                if !land.city_allowed(at.0, at.1, false)
                    || land.wet(at.0, at.1)
                    || (land.ground(at.0, at.1) * 8 - h).abs() > 96
                {
                    return None;
                }
                road.entry(at).or_insert(h);
            }
        }
    }
    Some(Site { city, center, road })
}

#[derive(Default)]
struct Planned {site:Option<Arc<Site>>,stats:PlanStats}
fn plan_with_city(seed:i64,cell:(i32,i32),land:&dyn Land,make_city:impl FnOnce()->Option<Arc<Vec<Placed>>>)->Planned {
    let mut out=Planned::default();
    let Some(city)=crate::jak_structures::village_spot(seed,cell) else{return out;};
    out.stats.candidates=1;
    if ![0,32,-32].into_iter().any(|dz|{let(lo,hi)=parcel_bounds(city,dz);reserved_for(seed,city,lo,hi)}) {
        out.stats.reserved=3;return out;
    }
    let h=land.probe_column(city.0,city.1).0;
    if !land.probe_city_allowed(city.0,city.1,h,false) {out.stats.city_biome=1;return out;}
    let mut make_city=Some(make_city);let mut blocks=None;
    for dz in [0,32,-32] {
        let Some(center)=parcel_center(seed,city,dz,land,&mut out.stats) else{continue;};
        if blocks.is_none() {
            blocks=make_city.take().expect("only one city build")();
            if blocks.is_none(){out.stats.city_missing=1;return out;}
        }
        if let Some(site)=connect_road(city,center,land,blocks.as_ref().unwrap()) {
            out.stats.accepted=1;out.site=Some(Arc::new(site));return out;
        }
        out.stats.road_rejected+=1;
    }
    out
}
fn legacy_plan(seed:i64,cell:(i32,i32),land:&dyn Land)->Arc<Planned>{
    type Legacy=Arc<OnceLock<Arc<Planned>>>;
    static CACHE:OnceLock<Mutex<HashMap<(i64,(i32,i32)),Legacy>>>=OnceLock::new();
    let entry={let mut cache=CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());if cache.len()>1024{if let Some(key)=cache.keys().next().copied(){cache.remove(&key);}}cache.entry((seed,cell)).or_default().clone()};
    entry.get_or_init(||Arc::new(plan_with_city(seed,cell,land,||crate::jak_structures::slums_road_anchors(seed,cell,land)))).clone()
}

/// Additional sites lie on a shared 320-block lattice. Its native models
/// cannot overlap one another; adjacent districts may share the same tower.
/// Accepted old parcels retain their exact coordinates and source cells.
fn slot_clear(seed:i64,city:(i32,i32),center:(i32,i32),land:&dyn Land)->bool{
    let lo=(center.0+tower::FOOTPRINT_MIN[0],center.1+tower::FOOTPRINT_MIN[1]);
    let hi=(center.0+tower::FOOTPRINT_MAX[0]-1,center.1+tower::FOOTPRINT_MAX[1]-1);
    let overlap=|a:(i32,i32),b:(i32,i32)|lo.0<=b.0&&hi.0>=a.0&&lo.1<=b.1&&hi.1>=a.1;
    let r=crate::jak_town::REACH+4;
    for cx in (lo.0-REACH).div_euclid(CELL)..=(hi.0+REACH).div_euclid(CELL){for cz in (lo.1-REACH).div_euclid(CELL)..=(hi.1+REACH).div_euclid(CELL){
        let Some(other)=crate::jak_structures::village_spot(seed,(cx,cz))else{continue;};
        if overlap((other.0-r,other.1-r),(other.0+r,other.1+r)){
            let h=land.probe_column(other.0,other.1).0;
            if land.probe_city_allowed(other.0,other.1,h,false)
                && crate::jak_structures::slums_exist(seed,(cx,cz),land){return false;}
        }
        // Compute only the legacy planner here: no recursive slot planning.
        let reaches_old=[0,32,-32].into_iter().any(|dz|{let(a,b)=parcel_bounds(other,dz);overlap(a,(b.0+40,b.1))&&reserved_for(seed,other,a,b)});
        if reaches_old {if let Some(old)=&legacy_plan(seed,(cx,cz),land).site{
            let c=old.center;
            if overlap((c.0+tower::FOOTPRINT_MIN[0],c.2+tower::FOOTPRINT_MIN[1]),(c.0+tower::FOOTPRINT_MAX[0]-1,c.2+tower::FOOTPRINT_MAX[1]-1))
                ||old.road.keys().any(|p|(lo.0..=hi.0).contains(&p.0)&&(lo.1..=hi.1).contains(&p.1)){return false;}
        }}
    }}
    let _=city;true
}

fn survey_slot_center(seed:i64,center:(i32,i32),land:&dyn Land,stats:&mut PlanStats)->Option<Pos>{
    let lo=(center.0+tower::FOOTPRINT_MIN[0],center.1+tower::FOOTPRINT_MIN[1]);
    let hi=(center.0+tower::FOOTPRINT_MAX[0]-1,center.1+tower::FOOTPRINT_MAX[1]-1);
    let(mut low,mut high)=(i32::MAX,i32::MIN);
    for x in (lo.0..=hi.0).step_by(16){for z in (lo.1..=hi.1).step_by(16){
        let(h,_wet)=land.probe_column(x,z);
        if !land.probe_city_allowed(x,z,h,false){stats.parcel_biome+=1;return None;}
        if ground_footprint(x-center.0,z-center.1){low=low.min(h);high=high.max(h);
            if high>MAX_BASE_Y||high-low>48{stats.elevation+=1;return None;}}
    }}
    if !slot_clear(seed,center,center,land){stats.reserved+=1;return None;}
    for x in lo.0..=hi.0{for z in lo.1..=hi.1{
        if !land.city_allowed(x,z,false){stats.parcel_biome+=1;return None;}
        if ground_footprint(x-center.0,z-center.1){let h=land.ground(x,z);low=low.min(h);high=high.max(h);
            if high>MAX_BASE_Y||high-low>48{stats.elevation+=1;return None;}}
    }}
    // Coastal plots use the same real basement/palace piers, extended to
    // the ocean-floor sample by foundation_range. Keep the entrance above
    // the water instead of rejecting every pond or coastal indentation.
    stats.parcels_viable+=1;Some((center.0,high.max(64),center.1))
}

fn slot_center(seed:i64,_city:(i32,i32),anchor:(i32,i32),land:&dyn Land,stats:&mut PlanStats)->Option<Pos>{
    // The choice belongs to the shared slot, independently of which city
    // requests it. Moving at most 72 blocks preserves a 16-block gap even
    // between opposite offsets in neighbouring 320-block slots.
    type Slot=Arc<OnceLock<(Option<Pos>,PlanStats)>>;
    static CACHE:OnceLock<Mutex<HashMap<(i64,(i32,i32)),Slot>>>=OnceLock::new();
    let entry={let mut cache=CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());
        if cache.len()>1024{if let Some(key)=cache.keys().next().copied(){cache.remove(&key);}}
        cache.entry((seed,anchor)).or_default().clone()};
    let (center,report)=entry.get_or_init(||{
        let mut report=PlanStats::default();
        for(dx,dz)in [(0,0),(72,0),(-72,0),(0,72),(0,-72),(72,72),(72,-72),(-72,72),(-72,-72)]{
            if let Some(center)=survey_slot_center(seed,(anchor.0+dx,anchor.1+dz),land,&mut report){return(Some(center),report);}
        }
        (None,report)
    });
    stats.add(report);*center
}

fn connect_slot(seed:i64,city:(i32,i32),center:Pos,land:&dyn Land,blocks:&[Placed],stats:&mut PlanStats)->Option<Site>{
    let side=if city.0<center.0{-1}else{1};
    for dz in [-28,0,28]{
        if let Some(site)=connect_slot_at(seed,city,center,(city.0-side*56,city.1+dz),land,blocks,stats){return Some(site);}
    }None
}
fn connect_slot_at(seed:i64,city:(i32,i32),center:Pos,end:(i32,i32),land:&dyn Land,blocks:&[Placed],stats:&mut PlanStats)->Option<Site>{
    let side=if city.0<center.0{-1}else{1};
    let end_height=blocks.iter().filter(|(p,_)|(p.0,p.2)==end).find_map(|(p,s)|{
        if *s=="jak:barrio_losa"{Some((p.1+1)*8)}else{s.strip_prefix("jak:barrio_losa_capa[layers=").and_then(|v|v.strip_suffix(']')).and_then(|v|v.parse::<i32>().ok()).map(|n|p.1*8+n)}
    })?;
    let outside_x=center.0+tower::FOOTPRINT_MAX[0]+8;
    let approach_x=city.0-side*80;
    let others:Vec<_>=((city.0-REACH).div_euclid(CELL)..=(city.0+REACH).div_euclid(CELL)).flat_map(|x|((city.1-REACH).div_euclid(CELL)..=(city.1+REACH).div_euclid(CELL)).filter_map(move|z|crate::jak_structures::village_spot(seed,(x,z)))).filter(|p|*p!=city).map(|p|(p,std::cell::OnceCell::<bool>::new())).collect();
    let mut skirts=[tower::FOOTPRINT_MIN[1]-8,tower::FOOTPRINT_MAX[1]+8];
    skirts.sort_by_key(|z|(center.2+z-city.1).abs());
    'route:for skirt in skirts{
      let skirt_z=center.2+skirt;
      let corners=[(center.0+67,center.2-10),(outside_x,center.2-10),(outside_x,skirt_z),(approach_x,skirt_z),(approach_x,end.1),end];
      let mut path=Vec::new();for pair in corners.windows(2){let(mut a,b)=(pair[0],pair[1]);while a!=b{path.push(a);a.0+=(b.0-a.0).signum();a.1+=(b.1-a.1).signum();}}path.push(end);
      if (end_height-center.1*8).abs()>2*(path.len()as i32-1){continue;}
      let mut road=BTreeMap::new();let mut previous=center.1*8;
      for(i,&(x,z))in path.iter().enumerate(){
        // Follow the terrain with a quarter-block maximum grade, while
        // retaining exact native/city endpoint heights. A fixed straight
        // ramp unnecessarily rejected otherwise usable undulating land.
        let remaining=2*(path.len()-1-i)as i32;
        // Stay at or above the Overworld sea surface when approaching a
        // bridge, rather than dipping into the riverbed before its bank.
        let ground=land.ground(x,z).max(63);
        let h=if i==0{previous}else{(ground*8).clamp((previous-2).max(end_height-remaining),(previous+2).min(end_height+remaining))};
        previous=h;
        for dx in -2..=1{for dz in -2..=1{let p=(x+dx,z+dz);
            if !land.city_allowed(p.0,p.1,false){stats.parcel_biome+=1;continue 'route;}
            if land.wet(p.0,p.1)&&h<63*8{stats.wet+=1;continue 'route;}
            if (land.ground(p.0,p.1)*8-h).abs()>192{stats.elevation+=1;continue 'route;}
            // A town can obstruct only a route which actually reaches it.
            // Eagerly constructing every neighbouring district made all
            // chunk workers wait for irrelevant shared tower dependencies.
            if others.iter().any(|(o,exists)|(p.0-o.0).abs()<=crate::jak_town::REACH+4
                &&(p.1-o.1).abs()<=crate::jak_town::REACH+4&&*exists.get_or_init(||{
                    let h=land.probe_column(o.0,o.1).0;
                    land.probe_city_allowed(o.0,o.1,h,false)
                        &&crate::jak_structures::slums_exist(seed,(o.0.div_euclid(CELL),o.1.div_euclid(CELL)),land)
                })){stats.reserved+=1;continue 'route;}
            road.entry(p).or_insert(h);
        }}
      }
      return Some(Site{city,center,road});
    }None
}

fn plan(seed:i64,cell:(i32,i32),land:&dyn Land)->Planned {
    let mut result=Planned::default();
    let Some(city)=crate::jak_structures::village_spot(seed,cell)else{return result;};
    result.stats.candidates=1;
    let h=land.probe_column(city.0,city.1).0;
    if !land.probe_city_allowed(city.0,city.1,h,false){result.stats.city_biome=1;return result;}
    // No detached fallback: every new tower has actual houses on four sides.
    if let Some(center)=crate::jak_structures::slums_palace(seed,cell,land){
        result.site=Some(Arc::new(Site{city,center,road:BTreeMap::new()}));
        result.stats.accepted=1;
    }else{result.stats.city_missing=1;}
    result
}

#[cfg(test)]mod distribution_regression{
 use super::*;
 #[test]
 #[ignore = "read-only natural terrain census; run separately from synthetic Land tests"]
 fn report_natural_city_tower_coverage(){
  let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
  let r=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
  let seed=12345;
  let gen=WorldGen::new(Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(r,seed).unwrap())).unwrap();
  let(mut towns,mut old,mut new)=(0,0,0);
  // Existing evidence: three coastal districts rejected by the first
  // expansion, plus the original Haven mechanics-probe district.
  for(x,z)in [(-24,-20),(-24,-19),(-24,-8),(-9,-18)]{
   let Some(city)=crate::jak_structures::village_spot(seed,(x,z))else{continue;};
   let h=gen.probe_column(city.0,city.1).0;
   if !gen.probe_city_allowed(city.0,city.1,h,false){continue;}
   let Some(blocks)=crate::jak_structures::village_cached(seed,(x,z),&gen,Style::Slums)else{continue;};
   if blocks.is_empty(){continue;}
   towns+=1;old+=usize::from(legacy_plan(seed,(x,z),&gen).site.is_some());
   let result=plan(seed,(x,z),&gen);new+=usize::from(result.site.is_some());
   println!("NATURAL_CITY city={city:?} tower={:?} stats={}",result.site.as_ref().map(|s|s.center),serde_json::to_string(&result.stats).unwrap());
   if result.site.is_none(){for anchor in [(x*CELL,z*CELL),((x+1)*CELL,z*CELL),(x*CELL,(z+1)*CELL),((x+1)*CELL,(z+1)*CELL)]{
    println!("NATURAL_SLOT anchor={anchor:?} center={:?}",slot_center(seed,city,anchor,&gen,&mut PlanStats::default()));
   }}
  }
  println!("NATURAL_COVERAGE towns={towns} old={old} new={new}");
  assert_eq!(towns,4);assert_eq!(new,towns,"the four existing coastal regression districts must all receive a tower");
 }
 struct Flat;
 impl Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
 #[test]fn dry_districts_receive_an_integrated_tower_without_overlapping_neighbours(){
  let seed=0x5ca7_ef17_2026i64;let(mut towns,mut before,mut after)=(0,0,0);let mut sites=Vec::new();
  for x in -4..4{for z in -4..4{if crate::jak_structures::village_spot(seed,(x,z)).is_none(){continue;}
   towns+=1;before+=usize::from(legacy_plan(seed,(x,z),&Flat).site.is_some());
   if let Some(site)=plan(seed,(x,z),&Flat).site{after+=1;if !sites.iter().any(|s:&Arc<Site>|s.center==site.center){sites.push(site);}}
  }}
  eprintln!("tower coverage: towns={towns} before={before} after={after}");
  assert!(towns>=15);assert!(after*100>=towns*90,"dry district coverage {after}/{towns}");assert_eq!(after,towns);
  for s in &sites {assert_eq!((s.center.0,s.center.2),s.city);assert!(s.road.is_empty());}
  for s in &sites{
   let bounds=s.affected_chunk_bounds();
   for(x,z)in [(bounds.0*16,bounds.1*16),(bounds.2*16+15,bounds.3*16+15)]{
    assert!((x-s.city.0).abs()<=REACH&&(z-s.city.1).abs()<=REACH,"generation lookup must cover the whole parcel and connecting road");
   }
  }
  for(i,a)in sites.iter().enumerate(){for b in &sites[i+1..]{
   let disjoint=a.center.0+tower::FOOTPRINT_MAX[0]<=b.center.0+tower::FOOTPRINT_MIN[0]
    ||b.center.0+tower::FOOTPRINT_MAX[0]<=a.center.0+tower::FOOTPRINT_MIN[0]
    ||a.center.2+tower::FOOTPRINT_MAX[1]<=b.center.2+tower::FOOTPRINT_MIN[1]
    ||b.center.2+tower::FOOTPRINT_MAX[1]<=a.center.2+tower::FOOTPRINT_MIN[1];
   assert!(disjoint,"physical towers overlap: {:?} {:?}",a.center,b.center);
  }}
 }
}
type Entry=Arc<OnceLock<Arc<Planned>>>;
fn cached_plan(seed:i64,cell:(i32,i32),land:&dyn Land)->Arc<Planned> {
    static CACHE:OnceLock<Mutex<HashMap<(i64,(i32,i32)),Entry>>>=OnceLock::new();
    let entry={let mut all=CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());
        if all.len()>256 {if let Some(k)=all.keys().next().copied(){all.remove(&k);}}
        all.entry((seed,cell)).or_default().clone()};
    let planned=entry.get_or_init(||{
        let _span=crate::frame_spans::span("native tower: plan city");
        let started=std::time::Instant::now();let result=Arc::new(plan(seed,cell,land));
        if started.elapsed().as_millis()>200{
            diag::info!(World,"Tower plan {:?}: {:.3}s, accepted {}, thread {:?}",cell,started.elapsed().as_secs_f64(),result.site.is_some(),std::thread::current().name());
            if crate::frame_spans::enabled(){
                static TRACES:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
                if TRACES.fetch_add(1,std::sync::atomic::Ordering::Relaxed)<4{diag::info!(World,"Tower planning caller: {}",std::backtrace::Backtrace::force_capture());}
            }
        }
        result
    }).clone();
    if let Some(site)=&planned.site {tower::remember(seed,site.center);}
    planned
}
fn cached(seed:i64,cell:(i32,i32),land:&dyn Land)->Option<Arc<Site>> {cached_plan(seed,cell,land).site.clone()}


/// Background far-terrain discovery shares the exact native site plan.
pub(crate) fn far_site(gen:&WorldGen,cell:(i32,i32))->Option<Pos>{
    cached(gen.terrain.seed,cell,gen).map(|site|site.center)
}

pub(crate) fn remember_near(gen: &WorldGen, pos: Pos) {
    for cx in (pos.0 - REACH).div_euclid(CELL)..=(pos.0 + REACH).div_euclid(CELL) {
        for cz in (pos.2 - REACH).div_euclid(CELL)..=(pos.2 + REACH).div_euclid(CELL) {
            if crate::jak_structures::village_spot(gen.terrain.seed, (cx, cz))
                .is_some_and(|(x, z)| (pos.0 - x).abs() <= REACH && (pos.2 - z).abs() <= REACH)
            {
                let _ = cached(gen.terrain.seed, (cx, cz), gen);
            }
        }
    }
}
pub(crate) fn cold_lookup(pos: Pos) {
    let _span=crate::frame_spans::span("native tower: cold source lookup");
    let Some((seed, registries)) = crate::original_places::current_world() else {
        return;
    };
    let Ok(terrain) =
        minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, seed)
    else {
        return;
    };
    if let Ok(gen) = WorldGen::new(Arc::new(terrain)) {
        remember_near(&gen, pos);
    }
}

/// Read-only locator for an actually eligible generated parcel, not a
/// fabricated debug tower. It uses the identical planner as chunk decoration.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct SearchProgress {
    pub ring:i32,
    pub ring_complete:bool,
    pub cells:usize,
    pub ring_stats:PlanStats,
    pub total:PlanStats,
}

pub fn nearest(registries:Arc<minecraftoss_core::Registries>,seed:i64,from:(i32,i32),reach:i32)->Option<Arc<Site>> {
    nearest_with_progress(registries,seed,from,reach,|_|true)
}

/// Reports each completed candidate and ring. Returning false cancels before
/// the next candidate; generation itself does not log or invoke callbacks.
pub fn nearest_with_progress(
    registries:Arc<minecraftoss_core::Registries>,seed:i64,from:(i32,i32),reach:i32,
    progress:impl FnMut(&SearchProgress)->bool,
)->Option<Arc<Site>> {
    nearest_eligible_with_progress(registries,seed,from,reach,|_|true,progress)
}

/// A migration guard may exclude already visited parcels while keeping the
/// same deterministic candidate cache and continuing to later rings.
pub fn nearest_eligible_with_progress(
    registries:Arc<minecraftoss_core::Registries>,seed:i64,from:(i32,i32),reach:i32,
    allowed:impl Fn(&Site)->bool,progress:impl FnMut(&SearchProgress)->bool,
)->Option<Arc<Site>> {
    let gen=WorldGen::new(Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries,seed).ok()?)).ok()?;
    search_sites(from,reach,allowed,progress,|cell|cached_plan(seed,cell,&gen))
}
fn search_sites(from:(i32,i32),reach:i32,allowed:impl Fn(&Site)->bool,
    mut progress:impl FnMut(&SearchProgress)->bool,mut lookup:impl FnMut((i32,i32))->Arc<Planned>)->Option<Arc<Site>> {
    let cell=(from.0.div_euclid(CELL),from.1.div_euclid(CELL));
    let mut report=SearchProgress::default();
    if !progress(&report){return None;}
    for ring in 0..=reach/CELL+1 {
        report.ring=ring;report.ring_complete=false;report.ring_stats=PlanStats::default();
        let mut found=Vec::new();
        for dx in -ring..=ring {for dz in -ring..=ring {
            if dx.abs().max(dz.abs())!=ring {continue;}
            let planned=lookup((cell.0+dx,cell.1+dz));
            report.cells+=1;report.ring_stats.add(&planned.stats);report.total.add(&planned.stats);
            if let Some(site)=&planned.site {
                if allowed(site){found.push(site.clone());}
                else {report.ring_stats.visited_rejected+=1;report.total.visited_rejected+=1;}
            }
            if !progress(&report){return None;}
        }}
        report.ring_complete=true;if !progress(&report){return None;}
        if let Some(site)=found.into_iter().min_by_key(|s| (s.center.0 as i64-from.0 as i64).pow(2)+(s.center.2 as i64-from.1 as i64).pow(2)) {return Some(site);}
    }
    None
}

const LAYERS: [&str; 7] = [
    "jak:barrio_losa_capa[layers=1]",
    "jak:barrio_losa_capa[layers=2]",
    "jak:barrio_losa_capa[layers=3]",
    "jak:barrio_losa_capa[layers=4]",
    "jak:barrio_losa_capa[layers=5]",
    "jak:barrio_losa_capa[layers=6]",
    "jak:barrio_losa_capa[layers=7]",
];
pub(crate) fn decorate(
    chunk: &mut Chunk,
    gen: &WorldGen,
    states: &mut HashMap<&'static str, BlockStateId>,
) {
    if !crate::jak_structures::natural_jak_world(gen){return;}
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    for cx in (x0 - REACH).div_euclid(CELL)..=(x0 + 15 + REACH).div_euclid(CELL) {
        for cz in (z0 - REACH).div_euclid(CELL)..=(z0 + 15 + REACH).div_euclid(CELL) {
            let Some((vx, vz)) = crate::jak_structures::village_spot(gen.terrain.seed, (cx, cz))
            else {
                continue;
            };
            // Skip unrelated chunks before consulting the shared district plan.
            if x0 + 16 + TREE_HALO < vx - REACH || x0 - TREE_HALO > vx + REACH
                || z0 + 16 + TREE_HALO < vz - REACH || z0 - TREE_HALO > vz + REACH {
                continue;
            }
            let Some(site) = cached(gen.terrain.seed, (cx, cz), gen) else {
                continue;
            };
            if !site.contains_chunk(x0, z0) {
                continue;
            }
            if !crate::jak_skate_tower_guard::allows(gen,site.affected_chunk_bounds()) {continue;}
            place_chunk(chunk, gen, states, &site);
        }
    }
}

fn mark_clearing(marks: &mut BTreeMap<(i32,i32),i32>, at: Pos) {
    marks.entry((at.0,at.2)).and_modify(|y|*y=(*y).min(at.1)).or_insert(at.1);
}

/// Only clearing metadata reaches into neighbouring chunks; block writes
/// remain strictly inside the chunk being generated.
fn clearing_marks(site: &Site, x0:i32, z0:i32) -> BTreeMap<(i32,i32),i32> {
    let mut marks=BTreeMap::new();
    for x in x0-TREE_HALO..x0+16+TREE_HALO {for z in z0-TREE_HALO..z0+16+TREE_HALO {
        if let Some((lo,_))=tower::air_column(x-site.center.0,z-site.center.2) {
            mark_clearing(&mut marks,(x,site.center.1+lo,z));
        }
        if let Some(&height)=site.road.get(&(x,z)) {
            mark_clearing(&mut marks,(x,height.div_euclid(8),z));
        }
    }}
    marks
}

fn mark_replaced_trunk(chunk:&Chunk,registries:&minecraftoss_core::Registries,at:Pos,ground:impl FnOnce()->i32,marks:&mut BTreeMap<(i32,i32),i32>) {
    let (x,z)=(at.0-chunk.pos.x*16,at.2-chunk.pos.z*16);
    if !(0..16).contains(&x)||!(0..16).contains(&z){return;}
    let state=chunk.block(x as usize,at.1,z as usize);
    let block=registries.blocks.block(registries.blocks.block_of(state));
    let Some(path)=block.name.as_str().strip_prefix("minecraft:")else{return;};
    if path.ends_with("_log")||path.ends_with("_wood")||matches!(path,"mushroom_stem"|"bamboo"|"bamboo_sapling") {
        mark_clearing(marks,(at.0,ground().min(at.1),at.2));
    }
}

const FOUNDATION:&str="jak:hv_t-citywide-wall-tile-01";
fn foundation_range(site:&Site,x:i32,z:i32,ground:i32)->std::ops::Range<i32> {
    let Some(local_floor)=tower::support_floor(x-site.center.0,z-site.center.2) else{return ground..ground;};
    let top=site.center.1+local_floor;
    ground.min(top)..top
}

fn place_chunk(
    chunk: &mut Chunk,
    gen: &WorldGen,
    states: &mut HashMap<&'static str, BlockStateId>,
    site: &Site,
) {
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let source = tower::source_origin();
    let mut placed = BTreeMap::<Pos, &'static str>::new();
    let mut clearings=clearing_marks(site,x0,z0);
    for x in x0..x0 + 16 {
        for z in z0..z0 + 16 {
            let (lx, lz) = (x - site.center.0, z - site.center.2);
            if tower::support_floor(lx,lz).is_some() {
                for y in foundation_range(site,x,z,gen.ground(x,z)) {placed.insert((x,y,z),FOUNDATION);}
            }
            if let Some((lo, hi)) = tower::air_column(lx, lz) {
                for y in site.center.1 + lo..site.center.1 + hi {
                    placed.insert((x, y, z), "minecraft:air");
                }
            }
            if tower::solid_base_column(lx,lz) {
                for y in 0..tower::core_height(lx,lz) {placed.insert((x,site.center.1+y,z),FOUNDATION);}
            }
            if let Some(&h) = site.road.get(&(x, z)) {
                let y = h.div_euclid(8);
                let rem = h.rem_euclid(8);
                for yy in gen.ground(x, z).min(y - 2)..y - 1 {
                    // Across water the street is a visible stone bridge,
                    // supported on real piers down to the sampled seabed.
                    // Keep the channel open between piers.
                    if !gen.wet(x,z){placed.insert((x, yy, z), "minecraft:dirt");}
                    else if x.rem_euclid(8)==0||z.rem_euclid(8)==0{placed.insert((x,yy,z),FOUNDATION);}
                }
                placed.insert((x, y - 1, z), "jak:barrio_losa");
                for yy in y..gen.ground(x, z).max(y) + 5 {
                    placed.insert((x, yy, z), "minecraft:air");
                }
                if rem > 0 {
                    placed.insert((x, y, z), LAYERS[(rem - 1) as usize]);
                }
            }
        }
    }
    let mut columns=HashMap::<(i32,i32),bool>::new();
    for x in x0-TREE_HALO..x0+16+TREE_HALO {for z in z0-TREE_HALO..z0+16+TREE_HALO {
        let(lx,lz)=(x-site.center.0,z-site.center.2);
        if (tower::FOOTPRINT_MIN[0]..tower::FOOTPRINT_MAX[0]).contains(&lx)
            && (tower::FOOTPRINT_MIN[1]..tower::FOOTPRINT_MAX[1]).contains(&lz) {
            let key=((lx+source[0]as i32).div_euclid(16)*16,(lz+source[2]as i32).div_euclid(16)*16);
            let own=(x0..x0+16).contains(&x)&&(z0..z0+16).contains(&z);
            columns.entry(key).and_modify(|needed|*needed|=own).or_insert(own);
        }
    }}
    let mut native = Vec::new();
    for ((sx, sz), own) in columns {
        let Some(column) = tower::column(sx, sz) else {
            continue;
        };
        if own { tower::prewarm_column(sx, sz); }
        for (&cell, piece) in &column.pieces {
            if piece.tris.is_empty(){continue;}
            let at = tower::to_world(site.center, cell);
            if (x0-TREE_HALO..x0+16+TREE_HALO).contains(&at.0)
                && (z0-TREE_HALO..z0+16+TREE_HALO).contains(&at.2) {
                mark_clearing(&mut clearings,at);
            }
            if !(x0..x0 + 16).contains(&at.0) || !(z0..z0 + 16).contains(&at.2) {
                continue;
            }
            mark_replaced_trunk(chunk,&gen.terrain.registries,at,||gen.ground(at.0,at.2),&mut clearings);
            let s = crate::jak_architecture::reusable_piece(piece).unwrap_or_else(|| match crate::jak_objects::slab_type(piece) {
                "top" => "jak:pieza_haven_rampa_modular[type=top,waterlogged=false]",
                "double" => "jak:pieza_haven_rampa_modular[type=double,waterlogged=false]",
                _ => "jak:pieza_haven_rampa_modular[type=bottom,waterlogged=false]",
            });
            native.push((at, s));
        }
    }
    // Clear complete canopies at the room/road before any authored pieces.
    let clearing: Vec<_> = clearings.into_iter().map(|((x,z),y)|((x,y,z),"minecraft:air")).collect();
    crate::jak_structures::clear_city_trees(chunk, &clearing, &gen.terrain.registries);
    for (at, s) in native {
        placed.insert(at, s);
    }
    // Ordinary foundation and modular fixtures own their collision separately
    // from the ramp meshes. Opening a gate never removes its supporting floor.
    for x in x0..x0+16 {for z in z0..z0+16 {
        if let Some(floor)=tower::ordinary_floor(x-site.center.0,z-site.center.2) {placed.insert((x,site.center.1-1,z),floor);}
    }}
    for (p,state) in tower::fixtures(){let at=(site.center.0+p.0,site.center.1+p.1,site.center.2+p.2);
        if (x0..x0+16).contains(&at.0)&&(z0..z0+16).contains(&at.2){placed.insert(at,state);}}
    // The settlement bell is a functional population anchor, not a prop.
    // Place it after palace clearing so the lower hall retains its town.
    if (site.center.0,site.center.2)==site.city
        &&(x0..x0+16).contains(&site.center.0)&&(z0..z0+16).contains(&site.center.2){
        placed.insert(site.center,"minecraft:bell[attachment=floor,facing=north,powered=false]");
    }
    for ((x, y, z), s) in placed {
        let id = *states.entry(s).or_insert_with(|| {
            gen.terrain
                .registries
                .blocks
                .parse_state(s)
                .unwrap_or(BlockStateId::AIR)
        });
        chunk.set_block_raw(
            (x - x0) as usize,
            y,
            (z - z0) as usize,
            id,
            &gen.terrain.registries,
        );
    }
}

pub fn diagnostic(seed: i64, feet: [f64; 3]) -> Vec<serde_json::Value> {
    tower::origins(seed).into_iter().filter(|c|(c.0 as f64-feet[0]).abs()<340.0&&(c.2 as f64-feet[2]).abs()<340.0)
        .map(|c|serde_json::json!({"center":[c.0,c.1,c.2],"height":tower::HEIGHT,"source_version":5,"source":9,"physical_circuit":true,
            "exterior":[c.0 as f64+94.5,c.1 as f64+0.05,c.2 as f64-10.0],
            "door":[c.0 as f64+tower::ACCESS[0],c.1 as f64+tower::ACCESS[1],c.2 as f64+tower::ACCESS[2]],
            "threshold":[c.0 as f64+tower::ARRIVAL[0],c.1 as f64+tower::ARRIVAL[1],c.2 as f64+tower::ARRIVAL[2]]})).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_legacy_parcels_do_not_survey_terrain_or_build_a_city() {
        struct Counted(std::cell::Cell<usize>);
        impl Counted {fn hit(&self){self.0.set(self.0.get()+1);}}
        impl Land for Counted {
            fn ground(&self,_:i32,_:i32)->i32{self.hit();70}
            fn wet(&self,_:i32,_:i32)->bool{self.hit();false}
            fn city_allowed(&self,_:i32,_:i32,_:bool)->bool{self.hit();true}
            fn probe_column(&self,_:i32,_:i32)->(i32,bool){self.hit();(70,false)}
        }
        let seed=0x7370_6172_6365_6c37i64;
        let cell=(-12..=12).flat_map(|x|(-12..=12).map(move|z|(x,z))).find(|&cell| {
            crate::jak_structures::village_spot(seed,cell).is_some_and(|city|
                [0,32,-32].into_iter().all(|dz|{let(lo,hi)=parcel_bounds(city,dz);!reserved_for(seed,city,lo,hi)}))
        }).expect("fixture has a city with three geometrically excluded parcels");
        let land=Counted(std::cell::Cell::new(0));
        assert!(plan_with_city(seed,cell,&land,||panic!("rejected legacy parcel must not build a city")).site.is_none());
        assert_eq!(land.0.get(),0,"a rejected parcel must not reach terrain sampling or district generation");
    }
    #[test]
    fn clearing_halo_removes_cross_chunk_crowns_and_only_replaced_trunks() {
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let r=minecraftoss_core::Registries::load(&paths).unwrap();
        let biome=r.biomes.id("minecraft:plains").unwrap();
        let log=r.blocks.parse_state("minecraft:spruce_log").unwrap();
        let leaf=r.blocks.parse_state("minecraft:spruce_leaves").unwrap();
        let mut left=Chunk::new(minecraftoss_core::ChunkPos::new(0,0),-64,384,biome);
        let mut right=Chunk::new(minecraftoss_core::ChunkPos::new(1,0),-64,384,biome);
        for y in 70..96 {left.set_block_raw(15,y,4,log,&r);}
        left.set_block_raw(15,96,4,leaf,&r);right.set_block_raw(0,96,4,leaf,&r);
        right.set_block_raw(2,90,4,log,&r);right.set_block_raw(15,96,15,leaf,&r);
        let site=Site{city:(1000,1000),center:(1000,70,1000),road:BTreeMap::from([((15,4),560)])};
        assert!(site.contains_chunk(16,0),"the halo-only neighbour must run cleanup");
        for (chunk,x) in [(&mut left,0),(&mut right,16)] {
            let marks=clearing_marks(&site,x,0);
            assert_eq!(marks.get(&(15,4)),Some(&70));
            let rows:Vec<_>=marks.into_iter().map(|((x,z),y)|((x,y,z),"minecraft:air")).collect();
            crate::jak_structures::clear_city_trees(chunk,&rows,&r);
        }
        assert_eq!(left.block(15,75,4),BlockStateId::AIR);
        assert_eq!(left.block(15,96,4),BlockStateId::AIR);
        assert_eq!(right.block(0,96,4),BlockStateId::AIR);
        assert_eq!(right.block(2,90,4),log,"a trunk outside the actual clearing stays intact");
        assert_eq!(right.block(15,96,15),leaf,"the distant forest stays intact");

        for y in 70..96 {left.set_block_raw(8,y,8,log,&r);}
        left.set_block_raw(4,90,4,log,&r);
        let mut marks=BTreeMap::new();mark_clearing(&mut marks,(8,85,8));
        mark_replaced_trunk(&left,&r,(8,85,8),||70,&mut marks);
        mark_clearing(&mut marks,(4,180,4));
        mark_replaced_trunk(&left,&r,(4,180,4),||panic!("high empty cells need no terrain survey"),&mut marks);
        assert_eq!(marks.get(&(8,8)),Some(&70));assert_eq!(marks.get(&(4,4)),Some(&180));
        let rows:Vec<_>=marks.into_iter().map(|((x,z),y)|((x,y,z),"minecraft:air")).collect();
        crate::jak_structures::clear_city_trees(&mut left,&rows,&r);
        assert_eq!(left.block(8,72,8),BlockStateId::AIR);
        assert_eq!(left.block(8,95,8),BlockStateId::AIR);
        assert_eq!(left.block(4,90,4),log,"a high native span must not clear a tree beneath it");
    }
    #[test]
    fn rejected_terrain_never_builds_the_district_and_parcels_are_not_resurveyed() {
        let seed=0x6761_7465_7375_7276i64;
        let cell=(-20..20).flat_map(|x|(-20..20).map(move|z|(x,z))).find(|&cell|
            crate::jak_structures::village_spot(seed,cell).is_some_and(|c|[0,32,-32].into_iter().any(|dz|{let(lo,hi)=parcel_bounds(c,dz);reserved_for(seed,c,lo,hi)}))).unwrap();
        struct Wet;
        impl Land for Wet {
            fn ground(&self,_:i32,_:i32)->i32{70}
            fn wet(&self,_:i32,_:i32)->bool{true}
            fn city_allowed(&self,_:i32,_:i32,_:bool)->bool{true}
        }
        let rejected=plan_with_city(seed,cell,&Wet,||panic!("wet parcels must reject before district generation"));
        assert!(rejected.site.is_none());assert!(rejected.stats.wet>0);assert_eq!(rejected.stats.parcels_viable,0);
        let city=crate::jak_structures::village_spot(seed,cell).unwrap();
        struct Counted(std::cell::Cell<usize>);
        impl Land for Counted {
            fn ground(&self,_:i32,_:i32)->i32{self.0.set(self.0.get()+1);70}
            fn wet(&self,_:i32,_:i32)->bool{false}
            fn probe_column(&self,_:i32,_:i32)->(i32,bool){(70,false)}
        }
        let land=Counted(std::cell::Cell::new(0));let before=std::cell::Cell::new(0);
        let built=plan_with_city(seed,cell,&land,||{
            before.set(land.0.get());Some(Arc::new(vec![((city.0-56,69,city.1-28),"jak:barrio_losa")]))
        });
        assert!(built.site.is_some());let ground_columns=(tower::FOOTPRINT_MIN[0]..tower::FOOTPRINT_MAX[0]).flat_map(|x|(tower::FOOTPRINT_MIN[1]..tower::FOOTPRINT_MAX[1]).map(move|z|(x,z))).filter(|&(x,z)|ground_footprint(x,z)).count();
        assert_eq!(before.get(),ground_columns,"all exact support columns precede district generation");
        assert!(land.0.get()-before.get()<4096,"connecting the road must not repeat the full parcel survey");
        assert_eq!(built.stats.accepted,1);assert_eq!(built.stats.parcels_viable,1);
    }
    #[test]
    fn crown_can_overhang_water_but_base_entry_and_full_model_biome_are_validated() {
        let seed=12345;
        let city=(0..30).flat_map(|x|(-10..10).map(move|z|(x,z))).find_map(|cell|{
            let c=crate::jak_structures::village_spot(seed,cell)?;let(lo,hi)=parcel_bounds(c,0);
            reserved_for(seed,c,lo,hi).then_some(c)
        }).unwrap();
        let center=(city.0-208,city.1);
        struct Marked {center:(i32,i32),wet:Option<(i32,i32)>,bad_biome:Option<(i32,i32)>,base:i32,low:Option<((i32,i32),i32)>}
        impl Land for Marked {
            fn ground(&self,x:i32,z:i32)->i32 {self.low.filter(|(p,_)|*p==(x-self.center.0,z-self.center.1)).map_or(self.base,|(_,h)|h)}
            fn wet(&self,x:i32,z:i32)->bool{self.wet==Some((x-self.center.0,z-self.center.1))}
            fn city_allowed(&self,x:i32,z:i32,_:bool)->bool{self.bad_biome!=Some((x-self.center.0,z-self.center.1))}
        }
        let mut land=Marked{center,wet:Some((80,60)),bad_biome:None,base:70,low:None};
        assert!(!ground_footprint(80,60));
        assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_some());
        for wet in [(0,0),(65,-10)] {
            assert!(ground_footprint(wet.0,wet.1));land.wet=Some(wet);
            assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_none(),"wet support at {wet:?}");
        }
        land.wet=None;land.bad_biome=Some((80,60));
        assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_none(),"overhangs still belong only to Haven biomes");
        land.bad_biome=None;land.base=MAX_BASE_Y;
        assert_eq!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).unwrap().1+tower::HEIGHT,320);
        land.base+=1;assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_none(),"highest native cell must remain below320");
        land.base=90;land.low=Some(((0,0),66));
        assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_some());
        land.low=Some(((0,0),65));assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_none(),"relief greater than24 rejects rather than flattening a mountain");
        let blocks=vec![((city.0-56,69,city.1-28),"jak:barrio_losa")];
        let valid_center=(center.0,70,center.1);
        land.base=70;land.low=None;land.wet=Some((73,-10));
        assert!(!ground_footprint(73,-10));
        assert!(parcel_center(seed,city,0,&land,&mut PlanStats::default()).is_some());
        assert!(connect_road(city,valid_center,&land,&blocks).is_none(),"water in the connecting street still rejects");
    }
    #[test]
    fn locator_skips_visited_sites_and_continues_to_the_next_ring() {
        let seed=0x6d69_6772_6174_6537i64;
        let cell=(-8..8).flat_map(|x|(-8..8).map(move|z|(x,z))).find(|&cell|crate::jak_structures::village_spot(seed,cell).is_some()).unwrap();
        let city=crate::jak_structures::village_spot(seed,cell).unwrap();
        let first=Arc::new(Site{city,center:(city.0-208,70,city.1),road:BTreeMap::new()});
        let second=Arc::new(Site{city:(city.0+CELL,city.1),center:(city.0+CELL-208,70,city.1),road:BTreeMap::new()});
        let lookup=|at|Arc::new(Planned{site:if at==cell{Some(first.clone())}else if at==(cell.0+1,cell.1){Some(second.clone())}else{None},stats:PlanStats::default()});
        let mut reports=Vec::new();
        let found=search_sites(city,CELL,|s|s.city!=first.city,|p|{reports.push(p.clone());true},lookup).unwrap();
        assert!(Arc::ptr_eq(&found,&second));
        assert_eq!(reports.last().unwrap().total.visited_rejected,1);
        assert_eq!(reports.last().unwrap().ring,1);assert!(reports.last().unwrap().ring_complete);
        assert!(reports.iter().any(|p|p.ring==0&&p.ring_complete&&p.total.visited_rejected==1));
        assert!(search_sites(city,CELL,|_|true,|_|false,|_|panic!("cancelled search must not plan a cell")).is_none());
    }
    #[test]
    fn migration_bounds_cover_model_road_and_cleanup_halo() {
        let site=Site{city:(0,0),center:(100,70,-100),road:BTreeMap::from([((300,-100),560)])};
        let (x0,z0,x1,z1)=site.affected_chunk_bounds();
        for (x,z) in [(100+tower::FOOTPRINT_MIN[0]-TREE_HALO,-100+tower::FOOTPRINT_MIN[1]-TREE_HALO),
            (100+tower::FOOTPRINT_MAX[0]-1+TREE_HALO,-100+tower::FOOTPRINT_MAX[1]-1+TREE_HALO),(300+TREE_HALO,-100)] {
            assert!((x0..=x1).contains(&x.div_euclid(16))&&(z0..=z1).contains(&z.div_euclid(16)));
        }
    }
    #[test]
    fn modular_foundation_supports_base_and_access_without_filling_crown_overhangs() {
        let site=Site{city:(1000,1000),center:(1000,70,1000),road:BTreeMap::new()};
        for (x,z) in [(0,0),(44,-5),(-5,64)] {
            let top=70+tower::support_floor(x,z).expect("base and access have support");
            assert_eq!(foundation_range(&site,1000+x,1000+z,top-5),top-5..top);
            assert!(foundation_range(&site,1000+x,1000+z,top+5).is_empty());
        }
        let ((x,_,z),_)=tower::fixtures().find(|(p,s)|p.1>=96&&*s!="minecraft:air"&&tower::support_floor(p.0,p.2).is_none())
            .expect("the enlarged crown and horns overhang the base");
        assert!(!ground_footprint(x,z));
        assert!(foundation_range(&site,1000+x,1000+z,40).is_empty(),"crown overhangs do not fill the world beneath");
    }
    #[test]
    fn modular_entry_opens_for_full_bodies_and_keeps_its_supporting_floor() {
        use crate::scene::{Block,HandcraftedScene};
        use minecraftoss_player::{World,collision::{Bounds,move_body}};
        use glam::DVec3;
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let r=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let states=Arc::new(crate::terrain::BlockStates::new(r.clone(),0,-64,384).unwrap());
        let mut scene=HandcraftedScene::streamed(states.clone());
        let origin=DVec3::new(38000.0,80.0,-38000.0);
        // The same modular fixtures used by place_chunk, including the new
        // five-panel gate under the front horn and independent half-slab floor.
        let mut gates=Vec::new();
        for (p,s) in tower::fixtures().filter(|(p,_)|(-10..=0).contains(&p.0)&&(27..=38).contains(&p.2)&&(4..=11).contains(&p.1)) {
            let at=(38000+p.0,80+p.1,-38000+p.2);
            let state=r.blocks.parse_state(s).unwrap();scene.set(at,states.block(state).cloned());
            if s.contains("krimson_porton_fence_gate"){gates.push(at);}
        }
        assert_eq!(gates.len(),5);
        for open in [false,true] {
            for &at in &gates {scene.set(at,Some(Block::new("jak:krimson_porton_fence_gate").with("facing","south").with("in_wall","false").with("powered","false").with("open",if open{"true"}else{"false"})));}
            for x in [-6.39,-5.5,-4.5,-3.61] {
                let feet=origin+DVec3::new(x,6.005,33.5);
                let moved=move_body(&scene,Bounds::standing(feet,1.2,2.6),DVec3::new(0.0,0.0,-4.0),true,0.6);
                if open {assert!((moved.z+4.0).abs()<1e-6,"open lane {x}: {moved:?}");}
                else {assert!(moved.z> -2.0,"closed lane {x}: {moved:?}");}
                for z in [29.5,30.0,31.0,32.0,33.5] {
                    for (dx,dz) in [(0.0,0.0),(-0.48,-0.48),(-0.48,0.48),(0.48,-0.48),(0.48,0.48)] {
                        let q=origin+DVec3::new(x+dx,6.0,z+dz);let at=(q.x.floor()as i32,85,q.z.floor()as i32);
                        assert!(World::collision_boxes(&scene,at).iter().any(|b|q.x>=at.0 as f64+b[0]-1e-6&&q.x<=at.0 as f64+b[3]+1e-6&&q.z>=at.2 as f64+b[2]-1e-6&&q.z<=at.2 as f64+b[5]+1e-6&&(at.1 as f64+b[4]-q.y).abs()<0.002),"support missing at {q:?}");
                    }
                }
            }
        }
    }
    struct Flat;
    impl Land for Flat {
        fn ground(&self, _: i32, _: i32) -> i32 {
            70
        }
        fn wet(&self, _: i32, _: i32) -> bool {
            false
        }
    }
    #[test]
    fn parcel_and_street_are_whole_and_do_not_take_a_neighbouring_city() {
        let (city, site) = (0..30)
            .flat_map(|x| (-10..10).map(move |z| (x, z)))
            .find_map(|cell| {
                let c = crate::jak_structures::village_spot(12345, cell)?;
                let blocks = vec![((c.0 - 56, 69, c.1 - 28), "jak:barrio_losa")];
                parcel_center(12345,c,0,&Flat,&mut PlanStats::default()).and_then(|center|connect_road(c,center,&Flat,&blocks)).map(|s| (c,s))
            })
            .expect("an eligible flat parcel");
        assert_eq!(site.center, (city.0 - 208, 70, city.1));
        assert!(site.center.0 + tower::FOOTPRINT_MAX[0] < city.0 - crate::jak_town::REACH);
        assert_eq!(site.road.get(&(city.0 - 56, city.1 - 28)), Some(&560));
        assert!(site.road.values().all(|&h| h == 560));
        struct Strip(i32);
        impl Land for Strip {
            fn ground(&self, _: i32, _: i32) -> i32 {
                70
            }
            fn wet(&self, _: i32, _: i32) -> bool {
                false
            }
            fn city_allowed(&self, x: i32, _: i32, _: bool) -> bool {
                x != self.0
            }
        }
        let blocks = vec![((city.0 - 56, 69, city.1 - 28), "jak:barrio_losa")];
        assert!(
            parcel_center(12345,city,0,&Strip(site.center.0+7),&mut PlanStats::default()).is_none(),
            "even a one-block biome strip rejects the whole tower"
        );
    }
    #[test]
    #[ignore = "read-only coordinates for the seed12345 physical skate probe"]
    fn report_seed12345_physical_tower() {
        let paths = minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let started=std::time::Instant::now();
        let site = nearest_with_progress(registries,12345,(-2792,-5531),6000,|p| {
            if p.ring_complete {println!("PHYSICAL_SEARCH {:.2}s {}",started.elapsed().as_secs_f64(),serde_json::to_string(p).unwrap());}
            true
        })
            .expect("eligible physical circuit parcel");
        println!(
            "PHYSICAL_TOWER city={:?} center={:?} entrance={:?}",
            site.city,
            site.center,
            site.entrance()
        );
        assert!(site.center.1 + tower::HEIGHT <= 320);
    }
}
