//! A compact, textured silhouette of the current buildable palace. Greedy
//! faces merge coplanar cells; the shaft stays narrow under the wide crown.
use super::{Atlas, Face, FarTile, FarVertex};
use crate::jak_items::BlockLook;
use crate::pack::ResourceId;
use std::collections::HashMap;

pub(super) fn materials(atlas:&Atlas)->HashMap<String,(Face,bool)>{
    crate::jak_items::block_looks().filter_map(|b|{
        if !(b.path.starts_with("palacio_")||b.path.starts_with("hv_t-citypal")){return None;}
        let texture=match b.look {
            BlockLook::All(t)|BlockLook::Slab{texture:t,..}=>t,
            BlockLook::Like{textures,..}|BlockLook::Shaped{textures,..}=>textures.first()?.1,
            _=>return None,
        };
        let id=ResourceId::parse(&format!("jak:block/{}",texture.to_lowercase())).ok()?;
        if !atlas.contains(&id){return None;}
        Some((format!("jak:{}",b.path),(Face{region:atlas.region(&id).map(|v|(v.clamp(0.,1.)*65535.).round()as u16),tint:[1.;3]},b.path=="palacio_ventana_luminosa")))
    }).collect()
}

pub(super) fn templates(materials:&HashMap<String,(Face,bool)>)->Vec<FarTile>{
    // A single canonical copy; no chunks or full city geometry are generated.
    let blocks:Vec<_>=crate::jak_city_arena::fixtures().filter_map(|(p,s)|
        materials.get(s.split('[').next().unwrap_or(s)).copied().map(|m|(p,m))).collect();
    if blocks.is_empty(){return Vec::new();}
    let mut min=[i32::MAX;3];let mut max=[i32::MIN;3];
    for &((x,y,z),_)in &blocks {for (a,v)in [x,y,z].into_iter().enumerate(){min[a]=min[a].min(v);max[a]=max[a].max(v);}}
    // Two-block cells are sub-pixel at the native streaming boundary, while
    // reducing tiny window/room faces and preserving the horn silhouette.
    const CELL:i32=2;
    min=min.map(|p|p.div_euclid(CELL)*CELL);
    let size=std::array::from_fn::<_,3,_>(|a|((max[a]-min[a])/CELL+1)as usize);
    let index=|p:[usize;3]|(p[2]*size[1]+p[1])*size[0]+p[0];
    let mut voxels=vec![0u16;size.iter().product()];let mut looks=Vec::new();
    let mut candidates:HashMap<usize,Vec<u16>>=HashMap::new();
    for ((x,y,z),m) in blocks{
        let id=if let Some(i)=looks.iter().position(|v|*v==m){i+1}else{looks.push(m);looks.len()};
        candidates.entry(index([((x-min[0])/CELL)as usize,((y-min[1])/CELL)as usize,((z-min[2])/CELL)as usize])).or_default().push(id as u16);
    }
    for (i,ids) in candidates{
        // Majority material, with repeated luminous window cells preserved.
        let mut chosen=ids[0];let mut most=0;
        for &id in &ids{let count=ids.iter().filter(|&&v|v==id).count();
            let weight=if looks[id as usize-1].1&&count>=2{count+8}else{count};
            if weight>most{most=weight;chosen=id;}
        }
        voxels[i]=chosen;
    }
    let empty=||FarTile{origin:[0,0],vertices:Vec::new(),indices:Vec::new(),trees:Vec::new(),bounds:[0.;6]};
    let mut parts=vec![empty()];
    for axis in 0..3{
        let(u,v)=((axis+1)%3,(axis+2)%3);let(w,h)=(size[u],size[v]);
        for slice in 0..=size[axis]{for positive in [false,true]{
            let mut mask=vec![0u16;w*h];
            for j in 0..h{for i in 0..w{
                let mut p=[0;3];p[u]=i;p[v]=j;
                let a=if slice>0{p[axis]=slice-1;voxels[index(p)]}else{0};
                let b=if slice<size[axis]{p[axis]=slice;voxels[index(p)]}else{0};
                mask[j*w+i]=if positive&&a!=0&&b==0{a}else if !positive&&b!=0&&a==0{b}else{0};
            }}
            for j in 0..h{let mut i=0;while i<w{
                let id=mask[j*w+i];if id==0{i+=1;continue;}
                let mut width=1;while i+width<w&&mask[j*w+i+width]==id{width+=1;}
                let mut height=1;
                'extend:while j+height<h{for x in i..i+width{if mask[(j+height)*w+x]!=id{break 'extend;}}height+=1;}
                for y in j..j+height{mask[y*w+i..y*w+i+width].fill(0);}
                if parts.last().unwrap().vertices.len()+4>60_000{parts.push(empty());}
                let part=parts.last_mut().unwrap();let base=part.vertices.len()as u32;
                let(face,lamp)=looks[id as usize-1];
                let shade=if lamp{1.}else if axis==0{0.6}else if axis==2{0.8}else if positive{1.}else{0.5};
                let axis_code=[1,0,2][axis]+if lamp{8}else{0};
                for(a,b)in [(i,j),(i+width,j),(i+width,j+height),(i,j+height)]{
                    let mut p=min;p[axis]+=slice as i32*CELL;p[u]+=a as i32*CELL;p[v]+=b as i32*CELL;
                    part.vertices.push(FarVertex{position:[p[0]as i16,(p[1]*8)as i16,p[2]as i16,0],colour:[(shade*255.)as u8,(shade*255.)as u8,(shade*255.)as u8,axis_code],region:face.region,floor:[0;4]});
                }
                part.indices.extend_from_slice(&[base,base+1,base+2,base,base+2,base+3]);
                i+=width;
            }}
        }}
    }
    parts
}

pub(super) fn positioned(template:&FarTile,center:(i32,i32,i32))->FarTile{
    let mut vertices=template.vertices.clone();let mut bounds=[f32::MAX;6];bounds[3..].fill(f32::MIN);
    for v in &mut vertices{v.position[1]+=(center.1*8)as i16;
        let p=[center.0 as f32+v.position[0]as f32,v.position[1]as f32/8.,center.2 as f32+v.position[2]as f32];
        for a in 0..3{bounds[a]=bounds[a].min(p[a]);bounds[a+3]=bounds[a+3].max(p[a]);}
    }
    FarTile{origin:[center.0,center.2],vertices,indices:template.indices.clone(),trees:Vec::new(),bounds}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn distant_tower_keeps_current_crown_horns_and_open_air_below_overhang(){
    // Synthetic atlas regions retain each original material independently.
    let mut next=1;let mats=crate::jak_items::block_looks().filter(|b|b.path.starts_with("palacio_")||b.path.starts_with("hv_t-citypal")).map(|b|{next+=1;(format!("jak:{}",b.path),(Face{region:[next;4],tint:[1.;3]},b.path=="palacio_ventana_luminosa"))}).collect();
    let parts=templates(&mats);assert!(!parts.is_empty());
    let vertices:Vec<_>=parts.iter().flat_map(|p|&p.vertices).collect();
    let count=vertices.len();assert!(count<250_000,"greedy silhouette became a full block mesh: {count}");
    assert!(vertices.iter().any(|v|v.position[1]>=168*8),"upper spire missing");
    assert!(vertices.iter().any(|v|v.position[0].abs()>=62&&v.position[1]>=96*8),"upper horns missing");
    assert!(vertices.iter().any(|v|v.colour[3]>=8),"night windows missing");
    // No heightfield wall from crown to ground at x=43, z=0.
    for part in &parts{assert!(part.vertices.len()<=60_000);for q in part.vertices.chunks_exact(4){
      let low=q.iter().map(|v|v.position[1]).min().unwrap();let high=q.iter().map(|v|v.position[1]).max().unwrap();
      assert!(!(low<18*8&&high>76*8),"far mesh filled the empty shaft surround");
    }}
    let placed=positioned(&parts[0],(20_000,70,-20_000));assert_eq!(placed.origin,[20_000,-20_000]);assert!(placed.bounds[1]>=70.);
    eprintln!("tower far geometry: {} parts, {} vertices, {} quads",parts.len(),count,count/4);
 }
}
