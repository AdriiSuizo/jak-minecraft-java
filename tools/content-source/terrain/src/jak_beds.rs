//! Place generated beds as complete pairs after doors and neighbouring lots.
//! Never edits a saved/player-authored world; both normal cities and exhibits use this.
use std::collections::{BTreeMap,BTreeSet,HashMap,HashSet,VecDeque};
use crate::jak_structures::Placed;
type Pos=(i32,i32,i32);
fn add(p:Pos,d:(i32,i32))->Pos{(p.0+d.0,p.1,p.2+d.1)}
fn bed(s:&str)->Option<(&str,u8,bool)>{
 let (id,props)=s.split_once('[')?;if !id.ends_with("_bed"){return None;}
 let way=if props.contains("facing=north"){0}else if props.contains("facing=east"){1}else if props.contains("facing=south"){2}else if props.contains("facing=west"){3}else{return None};
 Some((id,way,props.contains("part=head")))
}
fn floor(s:&str)->bool{
 // Only authored walkable floors, not a nearby wall, table or empty slab volume.
 if s.contains("type=bottom")||s.contains("escalera"){return false;}
 ["suelo","azotea","losa","_arena","_ground","_floor","planks","stone","concrete"].iter().any(|v|s.contains(v))
}
fn clear(c:&HashMap<Pos,&str>,p:Pos)->bool{c.get(&p).is_some_and(|s|*s=="minecraft:air")}
fn doorway(c:&HashMap<Pos,&str>,p:Pos)->bool{
 [(1,0),(-1,0),(0,1),(0,-1)].into_iter().any(|d|c.get(&add(p,d)).is_some_and(|s|s.contains("_puerta")||s.contains("_door[")))
}
fn usable(c:&HashMap<Pos,&str>,a:Pos,b:Pos)->bool{
 [a,b].into_iter().all(|p|!doorway(c,p)&&clear(c,p)&&clear(c,(p.0,p.1+1,p.2))&&c.get(&(p.0,p.1-1,p.2)).is_some_and(|s|floor(s)))
 && [a,b].into_iter().any(|p|[(1,0),(-1,0),(0,1),(0,-1)].into_iter().any(|d|{
  let q=add(p,d);q!=a&&q!=b&&clear(c,q)&&clear(c,(q.0,q.1+1,q.2))&&c.get(&(q.0,q.1-1,q.2)).is_some_and(|s|floor(s))
 }))
}
pub(crate) fn arrange(out:&mut Vec<Placed>){
 let mut cells:HashMap<_,_>=out.iter().copied().collect();
 let mut intended=BTreeSet::new();
 for (&p,&s) in &cells {if let Some((id,way,head))=bed(s){
  let d=crate::jak_facades::dir_step(way);
  intended.insert((if head{add(p,(-d.0,-d.1))}else{p},id.to_owned(),way));
 }}
 // Remove only surviving generated bed cells; keep overwritten walls/doors.
 let old:Vec<_>=cells.iter().filter_map(|(&p,&s)|bed(s).map(|_|p)).collect();
 let mut edits=BTreeMap::new();
 for p in old {cells.insert(p,"minecraft:air");edits.insert(p,"minecraft:air");}
 for (origin,id,way) in intended {
  // Search only the connected room, never through a partition into a neighbour.
  let other=add(origin,crate::jak_facades::dir_step(way));
  let mut room=HashSet::new();let mut queue=VecDeque::from([origin,other]);
  while let Some(p)=queue.pop_front(){
   if (p.0-origin.0).abs()>4||(p.2-origin.2).abs()>4||room.contains(&p)
    ||!clear(&cells,p)||!clear(&cells,(p.0,p.1+1,p.2))
    ||!cells.get(&(p.0,p.1-1,p.2)).is_some_and(|s|floor(s)){continue;}
   room.insert(p);for d in [(1,0),(-1,0),(0,1),(0,-1)]{queue.push_back(add(p,d));}
  }
  let mut candidates=Vec::new();
  for dx in -3i32..=3 {for dz in -3i32..=3 {for turn in 0..4u8 {
   candidates.push((dx.abs()+dz.abs(),turn,dx,dz));
  }}}
  candidates.sort_unstable();
  for (_,turn,dx,dz) in candidates {
   let facing=(way+turn)%4;let a=(origin.0+dx,origin.1,origin.2+dz);let b=add(a,crate::jak_facades::dir_step(facing));
   if !room.contains(&a)||!room.contains(&b)||!usable(&cells,a,b){continue;}
   for (p,part) in [(a,"foot"),(b,"head")] {
    let state=crate::jak_facades::leak(format!("{id}[facing={},occupied=false,part={part}]",crate::jak_facades::dir_name(facing)));
    cells.insert(p,state);edits.insert(p,state);
   }
   break;
  }
 }
 out.extend(edits);
}

#[cfg(test)]mod tests{
 use super::*;
 fn verify(out:Vec<Placed>)->usize{
  let c:HashMap<_,_>=out.into_iter().collect();let mut count=0;
  for (&a,&s) in &c {if let Some((id,w,head))=bed(s){
   let d=crate::jak_facades::dir_step(w);let b=add(a,if head{(-d.0,-d.1)}else{d});
   assert_eq!(c.get(&b).and_then(|s|bed(s)),Some((id,w,!head)),"broken bed at {a:?}");
   assert!(c.get(&(a.0,a.1-1,a.2)).is_some_and(|s|floor(s)),"no floor at {a:?}");
   assert!(clear(&c,(a.0,a.1+1,a.2)),"blocked sleeper at {a:?}");
   if !head{count+=1;}
  }}count
 }
 #[test]fn beds33_relocates_orphan_without_overwriting_door(){
  let mut room=Vec::new();for x in 0..6{for z in 0..6{
   room.push(((x,0,z),"jak:barrio_suelo"));for y in 1..3{room.push(((x,y,z),"minecraft:air"));}
  }}
  room.push(((2,1,2),"minecraft:red_bed[facing=east,occupied=false,part=foot]"));
  room.push(((3,1,2),"minecraft:oak_door[facing=north,half=lower,hinge=left,open=false,powered=false]"));
  arrange(&mut room);assert_eq!(verify(room.clone()),1);
  let c:HashMap<_,_>=room.iter().copied().collect();assert!(c[&(3,1,2)].contains("oak_door"));
  arrange(&mut room);assert_eq!(c,room.into_iter().collect());
 }
 #[test]fn beds33_all_rotated_house_archetypes_have_complete_supported_beds(){
  for way in 0..4 {for n in 0..crate::jak_barrio::ARCHETYPE_NAMES.len() {
   for (w,d) in [(6,6),(9,8),(12,9)] {
    let haven=crate::jak_barrio::building((0,70,0),way,crate::jak_barrio::archetype(n,w,d));
    assert!(verify(haven)>0,"haven archetype {n} {w}x{d} facing {way}");
    let spargus=crate::jak_spargus::building((40,70,0),way,crate::jak_spargus::archetype(n,w,d));
    assert!(verify(spargus)>0,"spargus archetype {n} {w}x{d} facing {way}");
   }
  }}
 }
 #[test]fn beds33_normal_cities_on_sloping_land_keep_complete_beds(){
  struct Hills;impl crate::jak_structures::Land for Hills{
   fn ground(&self,x:i32,z:i32)->i32{70+(x.div_euclid(32)+z.div_euclid(48)).rem_euclid(4)}
   fn wet(&self,_:i32,_:i32)->bool{false}
  }
  for (seed,turn) in [(3,0),(41,1),(12345,3)] {
   assert!(verify(crate::jak_barrio::village(seed,(500,-300),70,turn,&Hills))>10);
   assert!(verify(crate::jak_spargus::town(seed,(500,-300),70,turn,&Hills))>10);
  }
 }
}
