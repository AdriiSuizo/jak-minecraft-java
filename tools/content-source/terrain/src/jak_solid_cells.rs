//! Closed convex construction modules clipped to Minecraft cells. Each cut
//! receives its own cap, so removing a neighbour never exposes a hollow shell.
use glam::{Vec2,Vec3};
use crate::jak_pieces::SourceTri;

fn cut(faces:Vec<Vec<Vec3>>,axis:usize,limit:f32,sign:f32)->Vec<Vec<Vec3>> {
    let mut out=Vec::new();let mut rim:Vec<Vec3>=Vec::new();let mut outside=false;let mut inside=false;
    for face in faces {
        let mut poly=Vec::new();
        for i in 0..face.len(){let a=face[i];let b=face[(i+1)%face.len()];
            let da=(a[axis]-limit)*sign;let db=(b[axis]-limit)*sign;
            outside|=da>0.000001;inside|=da < -0.000001;
            if da<=0.000001{poly.push(a);}
            if da.abs()<=0.000001 && !rim.iter().any(|q|q.distance_squared(a)<1e-10){rim.push(a);}
            if (da<0. && db>0.)||(da>0. && db<0.) {
                let mut p=a.lerp(b,da/(da-db));p[axis]=limit;poly.push(p);
                if !rim.iter().any(|q|q.distance_squared(p)<1e-10){rim.push(p);}
            }
        }
        poly.dedup_by(|a,b|a.distance_squared(*b)<1e-10);
        if poly.len()>2{out.push(poly);}
    }
    if outside && inside && rim.len()>2 {
        let center=rim.iter().copied().sum::<Vec3>()/rim.len() as f32;
        let u=(axis+1)%3;let v=(axis+2)%3;
        rim.sort_by(|a,b|(a[v]-center[v]).atan2(a[u]-center[u]).total_cmp(&(b[v]-center[v]).atan2(b[u]-center[u])));
        out.push(rim);
    }
    out
}
pub(super) fn emit(faces:Vec<Vec<Vec3>>,texture:u16,out:&mut Vec<SourceTri>){
    let lo=faces.iter().flatten().fold(Vec3::splat(f32::INFINITY),|a,&b|a.min(b));
    let hi=faces.iter().flatten().fold(Vec3::splat(f32::NEG_INFINITY),|a,&b|a.max(b));
    for x in lo.x.floor() as i32..hi.x.ceil() as i32 {
      for y in lo.y.floor() as i32..hi.y.ceil() as i32 {
       for z in lo.z.floor() as i32..hi.z.ceil() as i32 {
        let cell=[x,y,z];let mut clipped=faces.clone();
        for axis in 0..3 {
            clipped=cut(clipped,axis,cell[axis] as f32,-1.);
            clipped=cut(clipped,axis,cell[axis] as f32+1.,1.);
        }
        let count=clipped.iter().map(Vec::len).sum::<usize>();if count==0{continue;}
        let center=clipped.iter().flatten().copied().sum::<Vec3>()/count as f32;
        for face in clipped {for i in 1..face.len()-1 {
            let mut p=[face[0],face[i],face[i+1]];
            let mut normal=(p[2]-p[0]).cross(p[1]-p[0]);
            if normal.length_squared()<1e-12{continue;}
            if normal.dot((p[0]+p[1]+p[2])/3.-center)<0.{p.swap(1,2);normal=-normal;}
            let n=normal.abs();let axis=if n.x>=n.y&&n.x>=n.z{0}else if n.y>=n.z{1}else{2};
            let uv=p.map(|v|match axis {0=>Vec2::new(v.z,v.y),1=>Vec2::new(v.x,v.z),_=>Vec2::new(v.x,v.y)}*0.5);
            out.push(SourceTri{p,uv,texture,color:[[128;4];3],repeat:[true;2],blend:false});
        }}
       }
      }
    }
}
pub(super) fn prism(lo:Vec3,hi:Vec3,texture:u16,out:&mut Vec<SourceTri>){
    let [x,y,z]=lo.to_array();let [a,b,c]=hi.to_array();
    let faces=[[[x,y,z],[a,y,z],[a,b,z],[x,b,z]],[[a,y,c],[x,y,c],[x,b,c],[a,b,c]],
        [[x,y,c],[x,y,z],[x,b,z],[x,b,c]],[[a,y,z],[a,y,c],[a,b,c],[a,b,z]],
        [[x,b,z],[a,b,z],[a,b,c],[x,b,c]],[[x,y,c],[a,y,c],[a,y,z],[x,y,z]]];
    emit(faces.into_iter().map(|f|f.into_iter().map(Vec3::from_array).collect()).collect(),texture,out);
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn architecture27_cut_solids_keep_caps_after_a_neighbour_is_removed(){
  let mut tris=Vec::new();prism(Vec3::ZERO,Vec3::new(2.,1.,1.),0,&mut tris);
  let n=tris.len();let mut source=crate::jak_pieces::Source::new(tris);source.own_cells(0..n);
  let column=crate::jak_pieces::cut_area(&source,(0,1),(0,0));
  for x in [0,1] {
   let cell=&column.pieces[&(x,0,0)];
   let volume:f32=cell.tris.iter().map(|t|{let p=t.p.map(Vec3::from_array);p[0].dot(p[2].cross(p[1]))/6.}).sum();
   assert!((volume-1.).abs()<0.001,"{x}: sealed volume {volume}");
   assert!(cell.tris.iter().any(|t|t.p.iter().all(|p|(p[0]-(1-x)as f32).abs()<1e-6)));
   assert!(!cell.collision_boxes().is_empty());
  }
 }
}
